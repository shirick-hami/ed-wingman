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
- Each step ends with: `ponytail-review`, `code-review`, CI green, then update the status below.

## Decisions (details in docs/PLAN.md)

Tauri 2 desktop app; Go + SQLite price server on a VPS behind Cloudflare (Spansh during build and test); EDDN upload on by default with an off switch, one random ID per install; Inara and EDSM sync optional, off by default; phone companion is a PWA linked through Google Drive app-data; ReShade add-on for exclusive Fullscreen in v2; no VR; Windows only; MIT; GitHub Actions; SignPath Foundation signing.

## Status

| Step | State |
|---|---|
| 1. Skeleton, journal reader, reducer, CI | Done except: verify against real journals (needs `test-journals/`), context-aware panels from `GuiFocus` |
| 2. Exploration panel, flight strip, unsold data, exobiology | Not started |
| 3–13 | Not started; see docs/PLAN.md "Build order" |

Measured memory, debug build, fixture: 6 MB app + about 150 MB WebView2 processes. Re-measure on a release build in step 12.
