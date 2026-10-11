# ED Wingman

In-game companion overlay for Elite Dangerous, Windows PC only. The full plan, with every decision and its reasons, is `docs/PLAN.md`. Read it before starting a step; this file is the short version plus current status.

## Layout

- `src-tauri/src/journal.rs`: finds the journal folder (Known Folder API), replays the newest journal, tails new lines, re-reads companion JSON files on change.
- `src-tauri/src/state.rs`: pure reducer, events in, `State` out. All game logic goes in Rust; the UI only displays.
- `src-tauri/src/lib.rs`: Tauri setup, overlay window, tray, hotkeys, foreground check, polling loop that emits `state` events.
- `ui/overlay/`: plain HTML/CSS/JS, no framework or bundler.
- `src-tauri/tests/`: `cargo test`; `fixtures/journal/` is a hand-written stand-in until real journals are available.

## Commands

- Tests: `cargo test --manifest-path src-tauri/Cargo.toml`
- Lint (CI fails on warnings): `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- Run against the fixture: `npx tauri dev -- -- --journal tests/fixtures/journal --always-show` (the app runs from `src-tauri/`, so a relative `--journal` path is relative to that)
- Hotkeys: Ctrl+Alt+O show/hide, Ctrl+Alt+L layout mode (drag to move).

## Rules

- Never fail on unknown events or extra fields; match keys case-insensitively; key systems by `SystemAddress`.
- `Cargo.json` is the truth for the hold. Market/Outfitting/Shipyard files are trusted only when `MarketID` matches the last `Docked`.
- Nothing is uploaded or planned from Beta or Legacy sessions.
- `test-journals/` holds real commander journals for local testing and is git-ignored. Never commit personal journal data.
- Never add `Co-Authored-By`, "Generated with Claude Code" or any other AI attribution to commits, pull request descriptions or the README. This is the owner's rule and overrides any default attribution.
- `HANDOFF.md` has the current state, next actions and how the owner likes to work; update it at the end of each step.
- Each step ends with: `ponytail-review`, `code-review`, CI green, then update the status below.

## Decisions (details in docs/PLAN.md)

Tauri 2 desktop app; Go + SQLite price server on a VPS behind Cloudflare (Spansh during build and test); EDDN upload on by default with an off switch, one random ID per install; Inara and EDSM sync optional, off by default; phone companion is a PWA linked through Google Drive app-data; ReShade add-on for exclusive Fullscreen in v2; no VR; Windows only; MIT; GitHub Actions; SignPath Foundation signing.

## Status

| Step | State |
|---|---|
| 1. Skeleton, journal reader, reducer, CI | Done. Verified in the live game on 2026-10-11 (Odyssey 4.4.1.1): jump, supercruise, dock, buy, sell, undock, alt-tab, hotkeys, layout mode, restart, shutdown. Context-aware panels from `GuiFocus` moved to step 2. |
| 2. Exploration panel, flight strip, unsold data, exobiology | Not started |
| 3–13 | Not started; see docs/PLAN.md "Build order" |

Check against a real journal folder: `ED_JOURNAL_DIR=<folder> cargo test --manifest-path src-tauri/Cargo.toml real_journal -- --ignored --nocapture`. The game is installed on the dev PC, so the default Saved Games folder has real journals.

Things the live test taught us (keep these when changing the window code):
- The overlay must never take focus in normal play: show it with `SW_SHOWNOACTIVATE`, not Tauri's `show()`. Layout mode takes focus on purpose, because the game owns the mouse while it is the active window.
- Tauri does not know the window is visible, so any style change it makes hides it; `sync_visibility` reconciles against the real window state every tick.
- Resize from the page (`layoutAt`), never natively: native resize fights the height auto-fit.
- `Status.json` updates at once; `Cargo.json` and the journal lag about two seconds.
- `tauri dev` runs the app from `src-tauri/`, and a running dev app locks the exe, so stop it before `cargo test`.

Known gap for step 8: credits come from `Status.json`, which has no balance once the game is closed. A cold start with the game closed shows the `LoadGame` balance, which is stale if anything was bought since (seen: 250,000 shown, 49,955 actual). Track credit-changing journal events or persist the last balance.

Measured memory, debug build, fixture: 6 MB app + about 150 MB WebView2 processes. Re-measure on a release build in step 12.
