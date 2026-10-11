pub mod journal;
pub mod state;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, Modifiers, ShortcutState};
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

/// Only where the overlay sits and how big it is; visibility is decided by the game being in front.
const SAVED: StateFlags = StateFlags::POSITION.union(StateFlags::SIZE);

type Shared = Arc<Mutex<state::State>>;

/// Layout mode: the overlay accepts the mouse so it can be dragged. Off = click-through.
static LAYOUT: AtomicBool = AtomicBool::new(false);
/// The user's show/hide choice. The overlay is actually shown only while the game is in front.
static USER_VISIBLE: AtomicBool = AtomicBool::new(true);

/// One tick checks the foreground window and a handful of file timestamps: cheap enough at 10 Hz.
const POLL: Duration = Duration::from_millis(100);

#[tauri::command]
fn get_state(shared: tauri::State<Shared>) -> state::State {
    shared.lock().unwrap().clone()
}

/// `--journal <dir>` overrides the Saved Games location (used for development and tests).
fn journal_dir() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == "--journal")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .or_else(journal::default_dir)
}

/// `--always-show` skips the foreground check, for development without the game.
fn always_show() -> bool {
    static ALWAYS: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| std::env::args().any(|a| a == "--always-show"));
    *ALWAYS
}

/// The game's window title is "Elite - Dangerous (CLIENT)".
#[cfg(windows)]
fn game_in_foreground() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};
    let mut buf = [0u16; 64];
    // SAFETY: buffer pointer and length match; a null foreground window yields 0 characters.
    let n = unsafe { GetWindowTextW(GetForegroundWindow(), buf.as_mut_ptr(), buf.len() as i32) };
    let title = String::from_utf16_lossy(&buf[..n.max(0) as usize]);
    // Our own window counts too: if it ever ends up in front, hiding it would start a show/hide loop.
    title.starts_with("Elite - Dangerous") || title == "ED Wingman"
}

#[cfg(not(windows))]
fn game_in_foreground() -> bool {
    true
}

/// Hidden when alt-tabbed away, so it never floats over the desktop.
fn should_show() -> bool {
    USER_VISIBLE.load(Ordering::SeqCst)
        && (always_show() || LAYOUT.load(Ordering::SeqCst) || game_in_foreground())
}

/// Makes the window's real visibility match `should_show()`.
///
/// Shown with SW_SHOWNOACTIVATE: Tauri's `show()` takes keyboard focus, which pulled it away from
/// the game and made the foreground check hide the overlay again, in a loop.
///
/// Compared against the real window state, not a remembered one: Tauri still believes the window
/// is hidden (it never showed it), so any style change it makes, such as toggling click-through,
/// rewrites the style without WS_VISIBLE and hides the window behind our back.
#[cfg(windows)]
fn sync_visibility(app: &AppHandle) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{IsWindowVisible, SW_HIDE, SW_SHOWNOACTIVATE, ShowWindow};
    let show = should_show();
    if let Some(Ok(hwnd)) = app.get_webview_window("main").map(|w| w.hwnd()) {
        // SAFETY: the handle belongs to our own live window.
        unsafe {
            if (IsWindowVisible(hwnd.0 as _) != 0) != show {
                ShowWindow(hwnd.0 as _, if show { SW_SHOWNOACTIVATE } else { SW_HIDE });
            }
        }
    }
}

#[cfg(not(windows))]
fn sync_visibility(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = if should_show() { w.show() } else { w.hide() };
    }
}

fn toggle_visible(app: &AppHandle) {
    USER_VISIBLE.fetch_xor(true, Ordering::SeqCst);
    // Act now rather than on the next tick, so the hotkey feels instant.
    sync_visibility(app);
}

/// Hands keyboard and mouse back to the game after layout mode.
#[cfg(windows)]
fn focus_game() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow};
    let title: Vec<u16> = "Elite - Dangerous (CLIENT)\0".encode_utf16().collect();
    // SAFETY: null class and a NUL-terminated title; a null result is checked before use.
    unsafe {
        let game = FindWindowW(std::ptr::null(), title.as_ptr());
        if !game.is_null() {
            SetForegroundWindow(game);
        }
    }
}

#[cfg(not(windows))]
fn focus_game() {}

/// Layout mode takes focus on purpose. While the game is the active window it owns the mouse:
/// it hides the cursor and treats every click as a control, so dragging the overlay fired weapons.
/// Making the overlay the active window frees the cursor and stops the game reading the mouse.
fn toggle_layout(app: &AppHandle) {
    let on = !LAYOUT.fetch_xor(true, Ordering::SeqCst);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.set_ignore_cursor_events(!on);
        let _ = w.set_focusable(on);
        if on {
            let _ = w.show();
            let _ = w.set_focus();
        }
        let _ = w.emit("layout", on);
    }
    if !on {
        focus_game();
    }
    sync_visibility(app);
    if !on {
        // Save on lock as well as on exit, so a crash or forced close keeps the layout.
        let _ = app.save_window_state(SAVED);
    }
}

fn start_reader(app: AppHandle, shared: Shared) {
    let dir = journal_dir();
    {
        let mut s = shared.lock().unwrap();
        s.journal_dir = dir.as_ref().map(|d| d.display().to_string()).unwrap_or_default();
        s.journal_found = dir.as_ref().is_some_and(|d| d.is_dir());
    }
    std::thread::spawn(move || {
        let mut reader = dir.map(journal::Reader::new);
        loop {
            sync_visibility(&app);

            let events = reader.as_mut().map(journal::Reader::poll).unwrap_or_default();
            if !events.is_empty() {
                let snapshot = {
                    let mut s = shared.lock().unwrap();
                    s.journal_found = true;
                    events.iter().for_each(|e| s.apply(e));
                    s.clone()
                };
                let _ = app.emit("state", snapshot);
            }
            std::thread::sleep(POLL);
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().with_state_flags(SAVED).build())
        .invoke_handler(tauri::generate_handler![get_state])
        .setup(|app| {
            let shared: Shared = Arc::default();
            app.manage(shared.clone());

            if let Some(w) = app.get_webview_window("main") {
                w.set_ignore_cursor_events(true)?;
                // Clicking the overlay in layout mode must not take focus from the game either.
                w.set_focusable(false)?;
            }

            // ponytail: fixed hotkeys; rebinding comes with the companion window.
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_shortcuts(["ctrl+alt+o", "ctrl+alt+l"])?
                    .with_handler(|app, shortcut, event| {
                        if event.state != ShortcutState::Pressed {
                            return;
                        }
                        let mods = Modifiers::CONTROL | Modifiers::ALT;
                        if shortcut.matches(mods, Code::KeyO) {
                            toggle_visible(app);
                        } else if shortcut.matches(mods, Code::KeyL) {
                            toggle_layout(app);
                        }
                    })
                    .build(),
            )?;

            let menu = Menu::with_items(
                app,
                &[
                    &MenuItem::with_id(app, "toggle", "Show / hide overlay  (Ctrl+Alt+O)", true, None::<&str>)?,
                    &MenuItem::with_id(app, "layout", "Layout mode  (Ctrl+Alt+L)", true, None::<&str>)?,
                    &MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?,
                ],
            )?;
            let mut tray = TrayIconBuilder::new()
                .tooltip("ED Wingman")
                .menu(&menu)
                .on_menu_event(|app, e| match e.id().as_ref() {
                    "toggle" => toggle_visible(app),
                    "layout" => toggle_layout(app),
                    "quit" => app.exit(0),
                    _ => {}
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;

            start_reader(app.handle().clone(), shared);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running ED Wingman");
}
