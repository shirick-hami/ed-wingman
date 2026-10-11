# ED Wingman: one in-game companion for Elite Dangerous (overlay, price server, phone app)

Project name **ED Wingman**, repository `ed-wingman` (chosen by the user). Before creating the repository, search GitHub and the Frontier forums to confirm no existing Elite tool uses the name.

**Platform: Windows PC only** (user decision). No Linux, macOS or console support; the desktop app, installer and CI builds target Windows alone.

## Context

Elite Dangerous players alt-tab between Inara, Coriolis, EDDiscovery, EDSM, Spansh, EDOMH and OD Explorer. The goal is one in-game overlay that shows the most useful part of each while flying, released publicly.

Elite has no mod API. Every tool in the list works the same way (confirmed by the second paste): the game writes **Journal logs** and status JSON files to disk, a companion app tails them, and community sites are fed from those via EDDN. So this project is a companion app that tails the journal and draws a transparent window over the game. It never touches game files or memory, which is the same footing as EDMC and EDDiscovery.

Decisions made with the user: in-game overlay for a public, MIT-licensed release; Tauri desktop app; owner-hosted Go price server (Spansh during build and test); EDDN upload on by default; optional Inara and EDSM sync; phone web app linked by Google sign-in; ReShade add-on for Fullscreen in v2; no VR; deployment by GitHub Actions; signing through SignPath Foundation.

**Milestones.** Steps 1 to 9 give a complete overlay on the owner's PC with remote data from Spansh (v0.1, personal use). Steps 10 to 12 make it public (v1.0). Step 13 adds Fullscreen (v2). Work proceeds one step at a time, each tested before the next.

**What the owner provides, and when.**
- Before step 1: Rust toolchain and Visual Studio C++ Build Tools installed; a real journal folder copied from a PC that has played the game; a public GitHub repository.
- Before step 6: own Inara and EDSM accounts and keys for live testing; Inara app registration if required.
- Before step 10: a VPS, a domain name, a Cloudflare account, Go installed, and the deploy secrets added to the repository.
- Before step 11: a Google Cloud project with the two OAuth client IDs.
- Before public release: SignPath Foundation application; courtesy contact with the EDDN maintainers about the new uploader.

## What is and is not possible

- A transparent window only shows over **Borderless or Windowed** mode. Fullscreen needs a different rendering path, covered in "Fullscreen and VR" below.
- **EDHM** (HUD recolor) is a shader injection and cannot be merged in. The overlay gets an accent-colour setting so it can match an EDHM theme.
- **VoiceAttack, HCS, EDCoPilot** are voice tools: out of scope.
- **Inara** has no public market-query API (its API is for syncing commander data, and scraping is against its terms). Trade data comes from Spansh during build and test and from the owner's server at release; both are fed by the same EDDN stream.
- Trade prices are only as fresh as the last commander who docked there. Every remote price shows its age.

## Stack

Chosen by the user for low memory use, after comparing Electron, Tauri, native Rust, native C++ and .NET.

| Part | Stack |
|---|---|
| Desktop overlay | **Tauri 2**: Rust core, UI as plain HTML/CSS/JS in Windows' built-in WebView2. No front-end framework or bundler. |
| Price server | **Go + SQLite**, one static binary |
| Phone companion | Plain HTML/JS installable web app |
| Fullscreen add-on (v2) | C++ ReShade add-on |
| Hosting | Budget VPS with Cloudflare's free tier in front; phone app on free static hosting; installers on GitHub Releases |

To install before starting: the Rust toolchain and the Visual Studio C++ Build Tools (Tauri needs both on Windows); Go when step 10 arrives. Node is already present and is only used for Tauri's command-line tool.

Repository root: `ed-wingman/`

| File | Job |
|---|---|
| `src-tauri/src/main.rs` | Overlay window (transparent, frameless, always-on-top, click-through), tray icon, global hotkeys, settings window; pushes state to the UI as Tauri events |
| `src-tauri/src/journal.rs` | Find journal dir, tail newest `Journal.*.log`, watch `Status.json`, `Cargo.json`, `Market.json`, `ShipLocker.json`; emit parsed events (`notify`, `serde_json`) |
| `src-tauri/src/state.rs` | Pure reducer: journal events in, app state out (system, bodies, materials, loadout, cargo, market) |
| `src-tauri/src/value.rs` | Body scan-value formula (the published community formula, about 20 lines) |
| `src-tauri/src/remote.rs` | HTTP client with disk cache, rate limit and identifying User-Agent; price queries (Spansh first, own server later) |
| `ui/overlay/` | `index.html`, `overlay.css`, `overlay.js`: the panels, rendering whatever state the core sends |
| `ui/settings/index.html` | Journal path, hotkeys, accent colour, panel position, per-panel on/off, Integrations page |
| `data/blueprints.json` | Engineering recipes vendored from EDCD `coriolis-data` (MIT, keep licence notice) |
| `src-tauri/tests/` | Sample journal fixture and `cargo test` reducer tests |

All logic lives in the Rust core; the web UI only displays. Secrets (Inara and EDSM keys, the Google refresh token) go in Windows Credential Manager through the `keyring` crate.

Default journal path: `%USERPROFILE%\Saved Games\Frontier Developments\Elite Dangerous`. It does not exist on this PC, so development runs against a fixture and the path is a setting.

## Panels and their data sources

1. **Exploration** (journal only, works offline and in undiscovered systems): `FSDJump`, `FSSDiscoveryScan` (body count), `Scan`, `SAAScanComplete`. Shows bodies scanned vs total, estimated value per body, terraformable / first-discovery / unmapped flags, and "worth mapping" highlights.
2. **Materials** (journal only): `Materials` snapshot plus `MaterialCollected`, `MaterialDiscarded`, `MaterialTrade`, `EngineerCraft`, `Synthesis`; `ShipLocker.json` for Odyssey. Shows counts against caps and how many rolls of pinned blueprints are affordable.
3. **Ship** (journal only): `Loadout` event already carries jump range, hull health, mass, cargo and rebuy, so no stat maths. Lists modules and engineering; hotkey opens the build in Coriolis or EDSY (gzip + base64 of the Loadout JSON, via the `flate2` crate).
4. **Trade**: `Market.json` for the docked station, `Cargo.json` for the hold; Spansh for best nearby sell prices for current cargo and a route suggestion. Cached, with data age shown.

**Sell alert (the headline trade feature).** The app knows where the player is (`Location`, `FSDJump`, `Docked`), what is in the hold (`Cargo.json`) and the ship's pad size (`Loadout`). When cargo changes, or after a jump with cargo aboard, it asks Spansh for stations within a set range that buy those goods and shows a toast: commodity, station, system, distance, price per tonne, total for the hold, and data age.

- Ranked by total credits for the actual hold, not headline price. Filtered by landing pad size, demand at least equal to the quantity carried, maximum data age, and maximum distance from arrival star. Fleet carriers excluded by default. All four limits are settings.
- A hotkey copies the system name to the clipboard for pasting into the galaxy map. The app cannot plot the route itself; nothing outside the game can.
- Queries fire only on cargo change or arrival, debounced and cached, never on a timer, to stay within Spansh's goodwill.

**Buy suggestion (next leg).** When docked, the trade panel also says what to load here and where to take it: top three options with commodity, quantity, destination, distance, profit per tonne, total profit and data age.

- Inputs from the journal: current station (`Docked`), free cargo space and jump range (`Loadout`, `Cargo.json`), pad size, and credit balance (`Status.json`), so it never suggests more than the player can afford or carry.
- Preferred source is Spansh's trade route planner, which takes exactly these inputs and does the ranking server-side in one request. Confirm its endpoint and that third-party use is welcome before relying on it. Fallback: compare the station's own `Market.json` stock against nearby buyers using the same query as the sell alert.
- `Market.json` is only written when the player opens the commodities screen, so the local-price half of the fallback appears only after that; the panel says so until then.
- Fires once per docking, cached per station.

The overlay is click-through, so it is driven by hotkeys: show/hide, next panel, open build in browser. Anything typed (searches, goals, blueprint pins, settings) happens in the companion window opened from the tray, or on the phone.

## Commander panel and goal planner

**Commander panel** (journal only): live balance (`Status.json`), net worth and lifetime figures (`Statistics`), ranks and progress (`Rank`, `Progress`), reputation, current ship and rebuy cost (`Loadout`), other owned ships (`StoredShips`).

**Goal planner**: the player enters a target balance. The app knows the current balance and the ship, and produces a plan to close the gap.

- **Trade plan, fully computed.** Find the best loop (A to B and back) for this ship: cargo capacity, pad size, jump range and available capital from the journal, margins from the price database. Estimate time per loop from jump count (distance over jump range), station distance from the arrival star and a fixed docking allowance. Simulate trip by trip, because early trips are limited by capital and profit compounds. Output: route, profit per loop, credits per hour, number of loops and estimated total time to reach the target.
- **Ranking is the player's choice:** fastest to target (credits per hour), fewest jumps, or safest.
- **Safety, from what is actually knowable:** system security level and anarchy status (in `FSDJump` and EDDN), whether the ship has shields (`Loadout`), cargo that is illegal at the destination, and a rebuy rule: never suggest a purchase that leaves the balance below the rebuy cost. The app cannot see pirates or other players, and the panel says so.
- **Mining plan, computed.** See "Mining" below; it is the second fully planned activity and is compared side by side with the trade plan.
- **Other activities, measured not guessed.** Combat, exploration and missions depend on skill, so the planner does not invent rates. It reads the player's own journal history (`MarketSell`, `MultiSellExplorationData`, `RedeemVoucher`, `MissionCompleted`, with timestamps) and shows their real credits per hour per activity beside the trade plan. Ship fit from `Loadout` (mining lasers, passenger cabins, weapons) decides which activities are listed as available.
- Every figure is labelled an estimate: prices fall as you sell in bulk and remote data may be stale.
- Loop search runs on Spansh's trade planner during build and test, and on our own server (one extra pair-search query) for release.
- **Ship upgrade advice (trade ships).** The planner also tests whether buying a bigger hauler gets to the target sooner.
  - `data/trade-ships.json`: about a dozen trade-capable ships, each with hull price, cost and cargo capacity of a standard trade fit, laden jump range, pad size and rank requirement. Generated from the vendored `coriolis-data`, not typed by hand, and regenerated when the game changes ships.
  - The player sets a **minimum safe reserve**. A purchase is allowed only if balance, plus trade-in value of the current ship (`HullValue` and `ModulesValue` from `Loadout`) when the player opts to sell it, minus ship and fit cost, still covers the reserve, the new ship's rebuy and enough capital to fill its hold.
  - The trip-by-trip simulation gains one branch: at each point where a candidate becomes affordable, compare time-to-target with and without switching, re-running the loop search for the new ship since pad size and range change the best route. With a dozen ships this is a small search.
  - Rank-locked ships are filtered using the player's `Rank`. Where to buy comes from shipyard data (EDDN `shipyard` messages in our database; Spansh station search during testing).
  - Output is one ordered plan: loop, number of runs, buy ship at station, next loop, and so on, with total time compared against the no-upgrade plan.
  - Limits stated in the panel: standard fits only, engineering ignored, trade ships only (mining and combat upgrades depend on skill, so they are not ranked).
- Not attempted: exploration route planning (needs a database of every body in the galaxy).

## Mining

**Planning (where to mine, where to sell).**
- Ship check from `Loadout`: mining lasers mean laser mining; seismic charge launcher plus abrasion blaster plus pulse wave analyser mean core mining; sub-surface missiles likewise. Warn on a missing refinery, prospector controller or collector controller, and suggest a limpet count for the hold size.
- Hotspots: ring type and reserve level from `Scan`, hotspot commodities and counts from `SAASignalsFound`. Both are relayed on EDDN, so our server stores them (confirm `SAASignalsFound` is in the current EDDN journal schema); Spansh body search during testing.
- Pick the best pair of ring and sell station for the fitted method: sell price times hold, minus travel time, preferring Pristine reserves. Sell stations must fit the pad and have demand of at least four times the hold, because price drops when a sale is a large share of demand (the multiple is a setting).
- Yield rate is the player's own: tonnes per hour measured from past `MiningRefined` events. Until there is history, a clearly labelled default is used.
- Output: ring, hotspot commodity, sell station, credits per full hold, holds needed and estimated time to target.
- Cannot be known from data: overlapping hotspots and exact asteroid locations. Not claimed.

**Live helper while mining.**
- `ProspectedAsteroid`: toast with the target commodity's percentage, remaining content and any core (motherlode), with a mine/skip verdict against a threshold setting.
- `MiningRefined` and `Cargo.json`: hold fill bar, limpets left, tonnes per hour and credits per hour for the session at the planned sell price.
- When the hold is full, the sell alert fires with the planned station, rechecked for a fresher price.

## Own price database (decided: Spansh for build and test, own server for public release)

Everything except galaxy-wide prices is already computed locally from the journal. Galaxy-wide prices can also be ours, but not on the player's PC: EDDN is a live stream with no history, so a client only learns about stations visited while it was listening, and a fresh install would know nothing. It needs one always-on server.

- `server/` (Go): subscribe to the EDDN ZeroMQ relay with a pure-Go ZeroMQ library, inflate each message, upsert `commodity` messages into SQLite (roughly 6M rows, well within SQLite), keep system coordinates from `journal` messages. Builds to one static binary run under systemd on the VPS, with Cloudflare in front for caching and a per-IP rate limit in the server so cost stays flat as users grow. Tests with `go test`.
- Core HTTP endpoints: best places to sell this cargo near these coordinates; best things to buy at this station with where to take them; best two-station loop near a point for a given ship (for the goal planner). All are SQL queries with a distance filter. Routes of three or more stops are not attempted. The mining, finder and colonisation lookups described in their own sections are further queries over the same database.
- Seed once from Spansh's public nightly station dump so day one is not empty; after that it is fed by EDDN only.
- `remote.rs` is the only client file that knows where prices come from, so the app is built against Spansh first (step 7) and switched to this server before public release (step 10). A public app sending every user's queries to one volunteer's site is the main reason to do this.
- Cost: a small always-on VPS and the duty to keep it up. If it is down, the trade panel shows "price service unavailable" and the journal-only panels keep working.

## Fullscreen and VR

`main.rs` gets a display-mode setting with three values. The first two show the same web page; the third is a separate drawing of the same state.

| Mode | How it is drawn | When |
|---|---|---|
| **Overlay** (default) | Transparent always-on-top window | v1. Borderless / Windowed |
| **Panel** | The same page in an ordinary opaque window | v1. Second monitor |
| **ReShade add-on** | Panels redrawn inside the game's frame from shared state | v2, step 13. Exclusive Fullscreen |

**VR is out of scope** (user decision).

**Exclusive Fullscreen: ReShade add-on (decided), step 13.** Nothing can draw over exclusive fullscreen except code inside the game's DirectX 11 renderer. Rather than ship our own injector, ReShade does the injecting and we ship an add-on (`reshade-addon/`, C++, built as `.addon64`).

- The add-on cannot host a web page, and WebView2 cannot hand over its rendered image, so the add-on has its own small UI. The Rust core writes the current state as JSON to a named shared-memory block; the add-on reads it and draws the panels with Dear ImGui, which ReShade provides to add-ons. This is a second, simpler rendering of the same panels and must be kept in step with the web UI.
- User requirement: ReShade installed for Elite **with full add-on support** (the separate download on reshade.me). The settings window detects it and offers to copy the add-on into the game folder.
- To confirm before coding: which add-on event draws every frame with the ReShade menu closed; that the global hotkeys still fire while the game holds exclusive fullscreen; coexistence with EDHM, which uses 3Dmigoto (a different DX11 hook) and not ReShade.
- Needs Visual Studio C++ Build Tools.

## Every user is a data contributor by default (EDDN upload built in, can be turned off)

Requirement: more users must mean fresher data. The app is already a journal reader, so rather than requiring a second app (EDMC), it uploads to EDDN itself. Spansh, Inara and EDSM all listen to EDDN, so every install improves the data this overlay reads back.

- New file `eddn.rs`: POST to the EDDN gateway for the `commodity`, `journal` (FSDJump, Scan, Docked, Location, SAASignalsFound), `outfitting` and `shipyard` schemas.
- **On by default, with an off switch** (final user decision). It sits on the Integrations page beside Inara and EDSM, which stay off by default. The first-run screen states what is sent and that it is anonymous, with the switch right there; the README repeats it. EDDN data carries no commander name and the uploader ID is hashed by the gateway, but messages do include system, station and timestamp, and the disclosure says so.
- The uploader ID the app sends is a random value, never the commander name, so even the EDDN gateway never receives a name. It is generated once per install and kept (user decision): EDDN and its listeners rely on a stable ID to isolate one faulty client. Confirm from the EDDN documentation how the gateway scrambles this ID and whether its scrambling key rotates, and word the disclosure to match.
- A bad uploader harms everyone and EDDN can block a misbehaving app by name, so the schema validation tests below are a release gate, not a nicety.
- EDDN's rules, which are the real work here: strip all `_Localised` and personal fields, add `StarSystem` / `StarPos` / `SystemAddress` to events that lack them, send `gameversion` / `gamebuild` in the header, never upload from beta or Legacy galaxies, queue and retry on failure.
- Develop against the `/test` schema variants only; switch to live schemas at release. Confirm the current schema versions and the gateway's duplicate handling (for users who also run EDMC) against the EDCD/EDDN repo before coding.
- Test: validate every outgoing message from the fixture journal against the published JSON schemas in `cargo test`.

**EDMC / EDDiscovery are not bundled and not required.** The built-in uploader covers data contribution for anyone who enables it. Bundling copies of their installers would ship versions that go stale (EDDN can refuse outdated uploaders), add licence duties (EDMC is GPL, EDDiscovery is Apache 2.0; check both before redistributing anything) and make their bugs our support load. With profile sync built in (below), nothing they do for this app's purposes is missing. First run only detects whether EDMC or EDDiscovery is already syncing, to avoid double submissions.

## Commander profile sync to Inara and EDSM (opt-in)

Presented as **optional integrations** (user decision; EDDN is on the same page but on by default): one "Integrations" page in settings with a separate switch per site, each off by default, and the app fully usable with none enabled. Detection of an existing EDMC or EDDiscovery install is shown on the same page.

Unlike EDDN, this is personal data, so it is off until the player turns it on per site and pastes in their own API key from that site's settings page.

- New file `sync.rs`. **EDSM**: post journal events to its journal endpoint with commander name and key; fetch EDSM's discard list first and skip the events it does not want; batch.
- **Inara**: post batched events (credits, ranks, reputation, travel, ship loadout, materials, cargo) to its API. Inara rate-limits hard, so events are queued and sent in a batch every couple of minutes, with a final flush on game exit.
- Keys are stored in Windows Credential Manager (encrypted by Windows for that user), never written to the plain settings file, never logged, and sent only to the site they belong to.
- If EDMC or EDDiscovery is detected with the same sync enabled, warn the player to use one or the other.
- Before coding: read both sites' current API documentation for endpoints, limits and terms. A public app must identify itself to both, and Inara expects new applications to be registered or approved; confirm that process and do it before release.
- Test: unit-test the event mapping against the fixture journal; live-test with the developer's own accounts only.

## Phone companion (installable web app, linked by Google sign-in)

Decisions: the overlay needs no account; Google sign-in exists only to link a player's PC app to their phone. The phone side is an installable web app (PWA), not a native Android app, so it also works on iPhone and needs no Play Store. The project owner hosts the price server only; no player's personal data is stored on it.

**How PC and phone talk without a relay server.** Both sign in to the same Google account and share two small files in that account's hidden Google Drive app-data folder (scope `drive.appdata`, which exposes only this app's own files, not the player's Drive). Storage and API quota belong to each player's own Google account.

- Desktop, new file `link.rs`: Google sign-in in the system browser (OAuth for installed apps, loopback redirect, PKCE); refresh token kept in Windows Credential Manager. Writes `state.json` when state changes, at most once every 30 seconds: last in-game location (system, station, docked or in flight, timestamp), balance, ship, cargo summary, active plan progress and a feed of the last 50 events (jumps, sell alerts, hold full, goal reached). Reads `goals.json` on the same cadence.
- Phone, new folder `phone/` (`index.html`, `app.js`, `manifest.json`, `sw.js`), hosted free as static files: Google sign-in, poll `state.json` every 30 seconds while open, show last location, commander summary and the event feed. A goal form (target balance, ranking preference, safe reserve) writes `goals.json`; the PC app picks it up and runs the goal planner, and the resulting plan appears on both.
- `goals.json` is input from outside the PC: validate types and ranges before use.
- Limits, stated in the app: updates arrive within about 30 seconds, not instantly; the phone shows nothing new while the PC app is closed; there are no notifications when the web app is closed (that would need a push server or a native app).
- Owner setup, one-time and free: a Google Cloud project, OAuth consent screen, one desktop and one web client ID. A public release needs Google's app verification once past the test-user limit. Confirm current Drive API quotas, the scope's classification and the verification steps from Google's documentation before coding.
- Test: sign in on both with the developer's account, replay the fixture journal, watch location and feed update on the phone, set a goal on the phone and see the plan appear in the overlay.

## Added features (from the review; all accepted by the user)

Each one plugs into an existing panel and reuses existing data, rather than standing alone.

**Flight strip: fuel and route** (step 2; journal only). A thin always-visible strip, separate from the panels.
- Fuel level and low-fuel flag from `Status.json`; tank size and whether a fuel scoop is fitted from `Loadout`; fuel burned per jump from recent `FSDJump` events, giving "fuel for about N more jumps".
- Plotted route from `NavRoute.json`: jumps remaining, and for each star ahead whether its class can be scooped. Warn when the remaining fuel will not reach the next scoopable star.
- Mission destinations and the planner's next stop are shown here too, so the strip is the one place that says where to go next.

**Unsold data at risk** (step 2; journal only). Running total of the estimated value of everything scanned since the last `SellExplorationData` / `MultiSellExplorationData`, plus unsold exobiology samples and unclaimed bounties and bonds, since all are lost on death. Cleared on sale or `Died`. Shown in the exploration panel and on the flight strip once it passes a threshold; the sell alert logic offers the nearest station that can take it.

**Exobiology helper** (step 2; Odyssey; journal plus one data file).
- Which bodies have biological signals and which genera, from `FSSBodySignals` and `SAASignalsFound`; ranked alongside mapping value in the exploration panel so "worth landing" sits next to "worth mapping".
- On the surface: three-sample progress per species from `ScanOrganic`, with the minimum distance required between samples for that genus and the current distance from the last sample, computed from latitude, longitude and planet radius in `Status.json`.
- Value per species from `data/organics.json`, a static table taken from a community source; confirm the source's licence and that values match the current game before vendoring. First-discovery bonus applied where the journal shows it.

**Mission and bounty tracker** (step 8; journal only, in the commander panel).
- Active missions from `Missions` at start-up and `MissionAccepted` / `MissionRedirected` / `MissionCompleted` / `MissionFailed` / `MissionAbandoned`: destination, reward, time left, with a warning toast before expiry.
- Mission cargo is the same record the sell alert uses to avoid selling it.
- Unclaimed bounties and combat bonds by faction from `Bounty`, `FactionKillBond` and `RedeemVoucher`, with the nearest place to cash them (uses the finder).
- Completed-mission and voucher income already feeds the planner's measured credits-per-hour figures.

**Ship as a goal** (step 8). The goal form, on PC and phone, accepts a ship instead of a number. Target balance is the ship's price plus a chosen fit level (hull only or standard fit), its rebuy and the safe reserve. `data/trade-ships.json` is widened to `data/ships.json` covering every ship, still generated from `coriolis-data`. When the goal is reached the finder gives the nearest shipyard selling it.

**Nearest seller finder** (step 7 on Spansh, then server endpoints in step 10).
- Nearest station selling a given module or ship, nearest material trader of each type, technology broker and Interstellar Factors, filtered by pad size and permits. The server gets these from EDDN `outfitting`, `shipyard` and the station-services list in `Docked` events.
- Used by other features instead of duplicating lookups: materials panel (nearest trader when a pinned blueprint is short), upgrade advice (where to buy the ship), bounty tracker (where to cash in), ship goal.
- Typed searches cannot happen in the click-through overlay, so the settings window becomes a **companion window** with tabs for Finder, Goals, Blueprints and Settings; the phone app gets the same Finder and Goals forms. Results appear in the overlay.

**Colonisation helper** (step 7, in the trade panel).
- What the player's construction site still needs, from the depot event the game writes when docked there, remembered per site because it only updates on docking. Confirm the exact event and field names against the current journal documentation; they were added in a recent game update.
- For each outstanding commodity, the best nearby source with enough stock for this ship's pad size, grouped into hold-sized trips, with trips remaining.
- Cargo bought for the site is marked so the sell alert does not offer to sell it.

## Corrections from the full feature review

Gaps found when checking every feature end to end. These are fixes to logic already planned, not new features, and each belongs to the step named.

- **State at start-up (step 1).** The game writes `Commander`, `LoadGame`, `Materials`, `Loadout`, `Cargo`, `Location` and `Statistics` once at session start. If the app starts mid-session it must replay the newest journal file from its first line, then tail. Also follow the game to a new journal file when it rotates.
- **More than one commander (step 1).** One journal folder can hold several commanders. Key all saved state, measured earning rates, sync settings and phone-link data by the commander ID in `LoadGame`.
- **Overlay behaviour (step 1).** Show the overlay only while the game window is in the foreground; otherwise it sits on top of the desktop. Add a layout mode, toggled by hotkey, in which the overlay accepts the mouse so panels can be dragged, resized and given their own opacity; hotkeys are rebindable because they can collide with game bindings. This moves "auto-hide when the game loses focus" out of the left-out list.
- **Cargo that must not be sold (step 7).** `Cargo.json` marks mission cargo and stolen goods. The sell alert and planners ignore mission cargo and limpets, and never send stolen goods to a normal market.
- **Places the player cannot go (steps 7 to 10).** Exclude permit-locked systems by default. Keep Legacy-galaxy data out entirely: the server drops EDDN messages whose game version is not the live galaxy.
- **Planner realism (step 8).** Repeating one loop drains the seller's stock and the buyer's demand, so constant profit for many loops is wrong. Cap each load by available stock, re-plan at every dock using the fresh prices the player's own `Market.json` gives, and show time to target as a range.
- **Server reliability (step 10).** The health endpoint reports the age of the last EDDN message, so a silently stalled listener fails the deploy check and an uptime monitor. Prices older than a set age are not returned. The API path carries a version (`/v1/`) so old app versions keep working after server changes. Reject obviously impossible prices.
- **Updates (step 12).** On start, compare the app version with the latest GitHub Release and show "update available". Outdated uploaders can be refused by EDDN, so this is needed, not optional.
- **Diagnostics (step 12).** A local log file and a "copy diagnostics" button; there is no telemetry.
- **A domain name is required (before step 10).** Cloudflare can only sit in front of a domain the owner controls, and Google's sign-in verification wants a home page and privacy policy on a verified domain. This is a small yearly cost not mentioned earlier.
- **Real journals are needed from step 1.** A hand-written fixture will miss real-world quirks. Get a journal folder from a PC that has played the game before building the reducer.

## From the user's "Local Player Data Reference" document

The user supplied a reference for the journal and companion files. Most of it confirms what is planned; these are the parts that were missing. The document notes that its own field lists for trade, ship and station files were not re-checked, so confirm those against the current journal manual when coding.

**Reader rules, all in step 1 (`journal.rs`, `state.rs`).**
- Resolve Saved Games with the Windows Known Folder API (`FOLDERID_SavedGames`), never a hardcoded `C:\Users` path; keep the manual override.
- Pick the newest journal by modified time or header timestamp, because two filename formats exist. Follow `Continued` into the next part file.
- Open files with shared read access. Process journal text only up to the last newline. If a companion JSON file fails to parse, wait 50 to 100 ms and retry. Debounce watcher events and skip repeats by the `timestamp` field.
- Ignore unknown events and extra fields; match keys case-insensitively. Flag beta and Legacy sessions from `Fileheader` and do not upload or plan from them.
- Key systems by `SystemAddress`, not name. Use symbol values for logic and `_Localised` values for display only.
- Treat `Cargo.json` as the truth for the hold; use buy and sell events only for history and profit tracking.
- Trust `Market.json`, `Outfitting.json` and `Shipyard.json` only when their `MarketID` matches the last `Docked` event.
- Read `Flags` as an unsigned 32-bit value. Track hull health from `Loadout`, then `HullDamage` and repair events.
- Game not running: detect by `Shutdown`, `Status.json` with `Flags: 0`, or the game process; the overlay hides and uploads pause.

**Small additions to existing features.**
- **Context-aware overlay (step 1).** `GuiFocus` and `Flags` from `Status.json` decide what is shown: hide panels while the galaxy map, system map or station services are open; show the exploration panel in FSS and SAA modes; show on-foot information when `Flags2` says on foot. The manual hotkeys still override.
- **Danger and legal warnings (step 2, flight strip).** Overheating, being interdicted, low fuel and "in danger" from `Flags`; `LegalState` of Wanted or a `Docked` event with `Wanted` or `ActiveFine` shows a warning, and the planner's safety ranking avoids sending a wanted player to high-security systems.
- **Engineer progress (step 3, materials panel).** `EngineerProgress` shows which engineers are unlocked, so pinned blueprints are marked available or locked. `Backpack.json` is read alongside `ShipLocker.json` for Odyssey materials.
- **Standing (step 8, commander panel).** `Reputation` shown as the named bands; `Powerplay` rank and merits and squadron name when present.
- **Console players.** No files exist for them; the first-run screen says so plainly.

**Fleet carrier panel** (accepted by the user; step 8, journal only, shown only to carrier owners).
- From `CarrierStats`: name and callsign, tritium, current and maximum jump range, balance and reserve, free space, docking access. Updated by `CarrierDepositFuel`, `CarrierBankTransfer`, `CarrierFinance`, `CarrierTradeOrder` and `CarrierJumpRequest` (destination and departure time, with a countdown).
- Upkeep runway: weeks of upkeep the balance covers. The journal does not give the weekly upkeep figure directly, so it is derived from active crew services using a static cost table; confirm the current costs before shipping, and label the figure an estimate.
- Carrier location comes only from `CarrierJumpRequest` or `CarrierJump` while aboard, and stats only refresh when the owner opens carrier management, so the panel always shows the age of its data.
- Ties into the rest: carrier balance is shown beside net worth in the commander panel; the goal planner can optionally count it; the trade panel lists the owner's own buy and sell orders.

**On-foot Odyssey helper** (accepted by the user; step 9, journal only).
- Vitals strip while on foot, from `Status.json`: oxygen, health, temperature and gravity, with `Flags2` warnings for low oxygen, low health, hot and cold.
- Current suit and loadout from `SuitLoadout` and `SwitchSuitLoadout`: suit, class, mods, weapons and their mods.
- Backpack and ship locker contents from `Backpack.json` and `ShipLocker.json`, grouped as items, components, consumables and data, with mission items marked so they are not sold or traded away.
- Upgrade shopping list: what the next suit or weapon class and each engineering mod still needs against what is held. Needs a static requirements table (`data/odyssey-upgrades.json`) from a community source; confirm its licence and currency before vendoring. The finder supplies the nearest place to buy or trade.
- The overlay switches to this view automatically when on foot (the context-aware rule above).

## Source control and deployment (GitHub Actions)

The user pushes to GitHub; nothing is deployed by hand. One repository rooted at `ed-wingman/` (not at `Claude Root`, which holds unrelated files). A public repository is assumed: Actions minutes and GitHub Pages are free for public repos, while a private one has a monthly minutes cap that Windows builds use up at a higher rate, and Pages needs a paid plan.

| Workflow | Trigger | Does |
|---|---|---|
| `ci.yml` | every push and pull request | `cargo test` on a Windows runner; `go test` on Linux once `server/` exists |
| `release-desktop.yml` | pushing a version tag (`v*`) | builds the Tauri installer on a Windows runner with the official Tauri action and attaches it to a GitHub Release; later also builds the ReShade add-on |
| `deploy-server.yml` | push to `main` changing `server/` | builds the Linux binary, copies it to the VPS over SSH, restarts the service, then calls a health endpoint and fails the run if it does not answer. The database file on the VPS is never touched by a deploy |
| `deploy-phone.yml` | push to `main` changing `phone/` | publishes the phone app to GitHub Pages |

- Secrets (VPS address and a deploy-only SSH key) are added by the user in the repository's Actions secrets, never committed. `.gitignore` covers build output and any local config.
- The deploy SSH key belongs to a VPS user that can only replace the binary and restart that one service.
- **Code signing: SignPath Foundation** (user decision), which signs open-source projects for free. Early builds are unsigned, with a note on the download page about the SmartScreen warning. Once the app works, apply; when accepted, add their signing step to `release-desktop.yml` between build and publish. Read their current terms before applying. Expected conditions: a public repository under an OSI-approved licence (MIT, chosen by the user; add `LICENSE` in step 1), releases built only by CI, a published code-signing policy in the README, and the certificate showing SignPath Foundation as publisher. Acceptance is their decision, so the fallback is staying unsigned or paying for a certificate.
- Set up `ci.yml` in step 1 so every later step is tested on push.

## Working method (tools now installed)

Each step uses the same loop, so quality checks are part of the step, not a later phase.

- **Before coding a step:** look up current APIs with Context7 (Tauri 2 and its plugins, `notify`, `serde`, `keyring`, Go SQLite and ZeroMQ libraries). For external services with no library (EDDN schemas, Spansh endpoints, Inara and EDSM APIs, journal manual) use the research skill and save findings under `docs/research/` in the repo, so later sessions read the file instead of re-researching.
- **While coding:** test-first for the reducer, value formula, planners and EDDN message builder (`mattpocock-skills:tdd`); rust-analyzer, gopls and clangd catch type errors as files are edited.
- **End of every step:** `ponytail-review` (anything removable), `code-review` (correctness), then `ci.yml` green before the next step.
- **Security, at the points where risk enters:** `owasp-security` when writing key storage and OAuth (steps 6 and 11); `supply-chain-risk-auditor` whenever `Cargo.lock` or `go.sum` gains dependencies; `stride-threat-model` on the server before it is public (step 10); `insecure-defaults:audit` and `security-audit` (secret scan of full git history) before v1.0.
- **Phone app (step 11):** `pwa-review` and `webapp-testing` (Playwright) for the sign-in, feed and goal round trip.
- **Release (step 12):** `compliance-auditor` drafts the privacy policy and EDDN disclosure from the actual data flows (Google verification and SignPath both need a published policy); `engineering:deploy-checklist` before each tagged release; `engineering:documentation` for README and code-signing policy.
- **Continuity:** claude-mem is disabled, so step 1 also creates `ed-wingman/CLAUDE.md` (via `init`) holding the decisions table, build-order status and pointers to this plan and `docs/research/`. Each step ends by updating its status line there. A fresh session reads it instead of this long conversation.
- **UI:** `frontend-design` and `ui-ux-pro-max` for the overlay and companion window look (dark, high-contrast, readable over a bright cockpit); `design:accessibility-review` on colour contrast and font sizes.

## Build order

Each step is usable on its own.

1. Skeleton: repository (`LICENSE`, `.gitignore`, `CLAUDE.md`, `ci.yml`), Tauri 2 app scaffolded with the Vanilla JavaScript template, overlay window (transparent, undecorated, always-on-top, `set_ignore_cursor_events` for click-through), `tauri-plugin-global-shortcut` for hotkeys, tray, journal tailer, reducer with fixture test.
2. Exploration panel, flight strip (fuel and route), unsold-data reminder, exobiology helper.
3. Materials panel with blueprint pinning.
4. Ship panel with Coriolis/EDSY export.
5. EDDN upload (`eddn.rs`), on by default with an off switch, before trade so that trade reads data the app also feeds.
6. Commander profile sync to Inara and EDSM (`sync.rs`), opt-in.
7. Trade panel with sell alert, buy suggestion, colonisation helper and the nearest seller finder (with the companion window). First task here is to confirm the current Spansh endpoints and usage expectations against the live site before writing `remote.rs`.
8. Commander panel, mission and bounty tracker, fleet carrier panel, and goal planner (trade loops, ship upgrade advice, ship as a goal).
9. Mining: live helper first (journal only), then the mining plan. Then the on-foot Odyssey helper.
10. Own price server (`server/`, hosted by the project owner), including ring, hotspot, outfitting, shipyard and station-services data and the finder endpoints; then point `remote.rs` at it. Required before any public release.
11. Phone companion (`link.rs`, `phone/`).
12. Settings window, installer (Tauri's bundler, NSIS), README with the Borderless requirement, Frontier non-affiliation disclaimer and third-party data credits.
13. (v2) State published to shared memory, and the ReShade add-on for exclusive Fullscreen that reads and draws it.

## Deliberately left out of v1

- VR, entirely.
- The ReShade add-on for Fullscreen (step 13, v2). Borderless covers it until then.
- A native Android app and phone notifications while the web app is closed.
- Player-run price servers. The owner hosts one; `remote.rs` takes the server address from settings, so self-hosting can be offered later without code changes.
- **Frontier CAPI login**: needs an OAuth client registered with Frontier and only adds fleet-carrier and remote-market data. The journal covers everything v1 shows.
- Multi-monitor placement, Linux and other non-Windows support (declined by the user), console players, languages other than English.

## Verification

- `cargo test`: replay the fixture journal through `state.rs` and assert system, body values, material counts and loadout.
- Measure the running overlay's memory in Task Manager (app plus its WebView2 processes) and record it; low memory is why this stack was chosen.
- `cargo tauri dev -- -- --journal src-tauri/tests/fixtures`, then append lines to the fixture and watch the overlay update live.
- Trade: check one known station's prices in the overlay against the Spansh website.
- Final check needs a PC with Elite installed: run the game in Borderless, jump, scan, dock, open the market, and confirm each panel. Elite's journal folder is not on this machine, so either the game gets installed here or a real journal folder is copied in for this step.
- Installer: build, install on a clean user profile, confirm first-run finds the journal folder or asks for it.
- Planner and alerts: unit tests with fixed price tables covering the rules that are easy to get wrong: mission and stolen cargo never offered for sale, rebuy and safe-reserve limits respected, loads capped by stock, permit-locked systems excluded.
- EDDN: every outgoing message validated against the published schemas in `cargo test`; live sends only to the `/test` schemas until release.
- Inara and EDSM sync: mapping unit-tested; live test with the owner's accounts and a check that the profile pages update.
- Server: `go test` on message ingestion and each query; after deploy, the health endpoint shows a recent EDDN message age; compare three stations' prices against Spansh.
- Phone: the sign-in, location, feed and goal round trip described in the phone section.
- Each step ends with its checks passing in `ci.yml` before the next step starts.

