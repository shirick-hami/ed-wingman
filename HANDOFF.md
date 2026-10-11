# Handoff: ED Wingman

Written 2026-10-11 at the end of the planning session and step 1. A new session should read this file, then `CLAUDE.md` (rules, commands, status), then the parts of `docs/PLAN.md` for the step it is starting.

## What this is

A Windows-only in-game overlay for Elite Dangerous that reads the game's journal files and shows exploration, materials, ship, trade, commander and mining information on top of the game. Later steps add a price server, a phone companion and a ReShade add-on for exclusive Fullscreen. Every product decision and its reason is in `docs/PLAN.md`; do not reopen them without the owner asking.

## Where things stand

Step 1 of 13 is finished and was tested in the live game (Odyssey 4.4.1.1) with the owner flying: jump, supercruise, docking, buying, selling, undocking, alt-tab, both hotkeys, layout mode, restart and clean shutdown all behaved correctly. All 302 journal lines of that session parsed.

What exists:

- Transparent, click-through, always-on-top overlay with a see-through cockpit-hologram look. Shown only while the game window is in front.
- Ctrl+Alt+O shows or hides it. Ctrl+Alt+L is layout mode: drag to move, corner grip to resize, size and position remembered.
- Journal reader and state reducer for commander, location, ship, credits, cargo and status. 9 tests plus an ignored real-journal replay check.
- CI on GitHub Actions: clippy with warnings as errors, then tests, on Windows.

## Next: step 2

Exploration panel, fuel and route strip, unsold-data reminder, exobiology helper, and the context-aware show/hide rules from `GuiFocus`. See "Panels and their data sources", "Added features" and "From the user's Local Player Data Reference document" in `docs/PLAN.md`.

Suggested order:

1. Look at real data first. The dev PC's Saved Games folder has a session with `FSDJump`, six `Scan` events, `FSSAllBodiesFound`, `NavRoute.json` and `FSSSignalDiscovered`. Run the replay command in `CLAUDE.md`.
2. Research and save to `docs/research/`: the published body scan-value formula, and the exobiology species value table with its licence. Both come from community sources and must be confirmed, not recalled.
3. Extend `state.rs` test-first: bodies scanned against the system's body count, value per body, first-discovery and mapped flags, fuel and route from `Status.json` and `NavRoute.json`.
4. Add the panels to `ui/overlay/`. The single panel that exists now is the seed of the always-visible flight strip.
5. Have the owner test it in the game before calling the step done.

## Known gaps

- Credits are wrong on a cold start with the game closed (details in `CLAUDE.md`). Fix in step 8, before the phone companion.
- The ship display-name table in `state.rs` was written from memory. Verify it against a current source in step 3.
- Hotkeys are fixed, not rebindable, and there is no settings window yet (step 12).
- The overlay resizes by scaling; it does not reflow. Revisit once step 2 adds more content.
- Memory measured 156 MB in a debug build, almost all WebView2. Measure a release build in step 12.
- `code-review` and `ponytail-review` have not been run on step 1; only clippy and self-review. Run both early in step 2.
- Borderless or windowed mode only. Exclusive Fullscreen needs the ReShade add-on (step 13).

## How the owner likes to work

- Explain things in plain language, short, with the reason. The owner is new to Elite Dangerous and to parts of this tooling; give in-game instructions one step at a time and wait for the result.
- Verify by running the real thing. Take a screenshot or read the journal instead of assuming, and say plainly when something could not be tested.
- Fix what a live test finds straight away, confirm with the owner, then commit.
- No `Co-Authored-By`, "Generated with" or other AI attribution in commits, pull requests or the README.
- UI work goes through the `frontend-design` and `ui-ux-pro-max` skills.
- Keep solutions small: reuse the platform and existing dependencies before adding code.
- Ask before anything outward-facing or irreversible (creating repositories, force-pushing, paid services).

## Things only the owner can do, none blocking step 2

- Search the Frontier forums to confirm no other tool is called ED Wingman.
- Before step 10: a small VPS and a domain name.
- Before step 11: a Google Cloud project for sign-in.
- Before public release: apply to SignPath Foundation, and contact the Spansh, Inara, EDSM and EDDN maintainers as listed in `docs/PLAN.md`.

## Dev machine

Rust (stable, MSVC), Visual Studio C++ Build Tools, Go, clangd, Node and the GitHub CLI are installed. Claude Code has the rust-analyzer, gopls and clangd language-server plugins and the Context7 documentation server. Use Context7 for Tauri and crate APIs before writing against them.
