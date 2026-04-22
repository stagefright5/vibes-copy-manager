use serde::Serialize;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};

const WIN_WIDTH: i32 = 420;
const WIN_HEIGHT: i32 = 520;
const CURSOR_OFFSET: i32 = 10;

fn main_window(app: &AppHandle) -> Result<tauri::WebviewWindow, String> {
    app.get_webview_window("main")
        .ok_or_else(|| "main window not found".into())
}

/// Position window near the cursor, clamped to screen edges.
fn position_at_cursor(win: &tauri::WebviewWindow) {
    let cursor = match win.cursor_position() {
        Ok(p) => p,
        Err(_) => {
            let _ = win.center();
            return;
        }
    };

    let cx = cursor.x as i32;
    let cy = cursor.y as i32;

    let (mon_x, mon_y, mon_w, mon_h) = match win.monitor_from_point(cursor.x, cursor.y) {
        Ok(Some(m)) => (
            m.position().x,
            m.position().y,
            m.size().width as i32,
            m.size().height as i32,
        ),
        _ => match win.primary_monitor() {
            Ok(Some(m)) => (
                m.position().x,
                m.position().y,
                m.size().width as i32,
                m.size().height as i32,
            ),
            _ => {
                let _ = win.center();
                return;
            }
        },
    };

    let mut x = cx + CURSOR_OFFSET;
    let mut y = cy + CURSOR_OFFSET;

    if x + WIN_WIDTH > mon_x + mon_w {
        x = cx - WIN_WIDTH - CURSOR_OFFSET;
    }
    if y + WIN_HEIGHT > mon_y + mon_h {
        y = cy - WIN_HEIGHT - CURSOR_OFFSET;
    }
    if x < mon_x {
        x = mon_x;
    }
    if y < mon_y {
        y = mon_y;
    }

    let _ = win.set_position(PhysicalPosition::new(x, y));
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
struct WindowShownPayload {
    should_focus_search: bool,
}

fn should_focus_search() -> bool {
    #[cfg(target_os = "linux")]
    {
        false
    }

    #[cfg(not(target_os = "linux"))]
    {
        true
    }
}

pub fn do_show(app: &AppHandle) -> Result<(), String> {
    let win = main_window(app)?;
    let should_focus_search = should_focus_search();

    position_at_cursor(&win);
    win.set_always_on_top(true).map_err(|e| e.to_string())?;
    win.show().map_err(|e| e.to_string())?;

    if should_focus_search {
        win.set_focus().map_err(|e| e.to_string())?;
    }

    let _ = app.emit(
        "window-shown",
        WindowShownPayload {
            should_focus_search,
        },
    );
    Ok(())
}

pub fn do_hide(app: &AppHandle) -> Result<(), String> {
    let _ = app.emit("window-hiding", ());
    let win = main_window(app)?;
    win.hide().map_err(|e| e.to_string())?;
    win.set_always_on_top(false).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn do_toggle(app: &AppHandle) -> Result<(), String> {
    let win = main_window(app)?;
    if win.is_visible().map_err(|e| e.to_string())? {
        do_hide(app)
    } else {
        do_show(app)
    }
}

#[tauri::command]
pub fn hide_window(app: AppHandle) -> Result<(), String> {
    do_hide(&app)
}

#[tauri::command]
pub fn show_window(app: AppHandle) -> Result<(), String> {
    do_show(&app)
}

#[tauri::command]
pub fn toggle_window(app: AppHandle) -> Result<(), String> {
    do_toggle(&app)
}

#[tauri::command]
pub fn paste_and_hide(app: AppHandle) -> Result<(), String> {
    do_hide(&app)?;

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(80));
        simulate_paste();
    });

    Ok(())
}

/// Cross-platform paste simulation.
fn simulate_paste() {
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;

        if let Err(e) = Command::new("wtype")
            .args(["-M", "ctrl", "-k", "v", "-m", "ctrl"])
            .status()
        {
            eprintln!("[paste] failed to run wtype: {e}");
        }
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let _ = Command::new("osascript")
            .args(["-e", "tell application \"System Events\" to keystroke \"v\" using command down"])
            .status();
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, Tauri's clipboard write is enough — the user
        // can paste with their normal Ctrl+V. We could use SendInput
        // via the windows crate but it requires extra deps. The
        // clipboard already holds the text.
    }
}
