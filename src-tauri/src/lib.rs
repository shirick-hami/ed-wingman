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

type Shared = Arc<Mutex<state::State>>;

/// Layout mode: the overlay accepts the mouse so it can be dragged. Off = click-through.
static LAYOUT: AtomicBool = AtomicBool::new(false);
/// The user's show/hide choice. The overlay is actually shown only while the game is in front.
static USER_VISIBLE: AtomicBool = AtomicBool::new(true);

const POLL: Duration = Duration::from_millis(250);

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
    std::env::args().any(|a| a == "--always-show")
}

/// The game's window title is "Elite - Dangerous (CLIENT)".
#[cfg(windows)]
fn game_in_foreground() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};
    let mut buf = [0u16; 64];
    // SAFETY: buffer pointer and length match; a null foreground window yields 0 characters.
    let n = unsafe { GetWindowTextW(GetForegroundWindow(), buf.as_mut_ptr(), buf.len() as i32) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize]).starts_with("Elite - Dangerous")
}

#[cfg(not(windows))]
fn game_in_foreground() -> bool {
    true
}

fn toggle_visible(_app: &AppHandle) {
    USER_VISIBLE.fetch_xor(true, Ordering::SeqCst);
}

fn toggle_layout(app: &AppHandle) {
    let on = !LAYOUT.fetch_xor(true, Ordering::SeqCst);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.set_ignore_cursor_events(!on);
        let _ = w.emit("layout", on);
    }
}

fn start_reader(app: AppHandle, shared: Shared) {
    let dir = journal_dir();
    {
        let mut s = shared.lock().unwrap();
        s.journal_dir = dir.as_ref().map(|d| d.display().to_string()).unwrap_or_default();
        s.journal_found = dir.as_ref().is_some_and(|d| d.is_dir());
    }
    let always = always_show();
    std::thread::spawn(move || {
        let mut reader = dir.map(journal::Reader::new);
        let mut shown = None;
        loop {
            // Hidden when alt-tabbed away, so it never floats over the desktop.
            let show = USER_VISIBLE.load(Ordering::SeqCst)
                && (always || LAYOUT.load(Ordering::SeqCst) || game_in_foreground());
            if shown != Some(show) {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = if show { w.show() } else { w.hide() };
                }
                shown = Some(show);
            }

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
        .invoke_handler(tauri::generate_handler![get_state])
        .setup(|app| {
            let shared: Shared = Arc::default();
            app.manage(shared.clone());

            if let Some(w) = app.get_webview_window("main") {
                w.set_ignore_cursor_events(true)?;
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
