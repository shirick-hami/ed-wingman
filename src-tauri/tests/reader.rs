use ed_wingman_lib::journal::Reader;
use ed_wingman_lib::state::{is_beta_or_legacy, State};
use serde_json::json;
use std::io::Write;
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/journal")
}

fn replay(dir: PathBuf) -> State {
    let mut state = State::default();
    Reader::new(dir).poll().iter().for_each(|e| state.apply(e));
    state
}

#[test]
fn fixture_replay_builds_current_state() {
    let s = replay(fixture_dir());

    assert_eq!(s.commander.name, "Test Pilot");
    assert!(s.odyssey && !s.beta_or_legacy && s.running);

    assert_eq!(s.location.system, "Eranin");
    assert_eq!(s.location.system_address, Some(2832631632594));
    assert!(s.location.docked);
    assert_eq!(s.location.station.as_deref(), Some("Azeban City"));
    assert_eq!(s.location.market_id, Some(128001536));

    assert_eq!(s.ship.kind, "type9");
    assert_eq!(s.ship.model, "Type-9 Heavy");
    assert_eq!(s.ship.cargo_capacity, Some(720));
    assert_eq!(s.ship.hull_health, Some(0.82));

    // Status.json overrides the LoadGame balance; Cargo.json is the hold's truth.
    assert_eq!(s.balance, Some(151_200_000));
    assert_eq!(s.cargo.count, 18);
    assert_eq!(s.cargo.items.len(), 3);
    assert_eq!(s.cargo.items[1].mission_id, Some(65397935));
    assert_eq!(s.cargo.items[2].display, "Ion Distributor");
}

/// Replays a real journal folder and prints the resulting state, for checking against the game.
/// `ED_JOURNAL_DIR=<folder> cargo test real_journal -- --ignored --nocapture`
#[test]
#[ignore = "needs ED_JOURNAL_DIR pointing at a real journal folder"]
fn real_journal_replays_without_panicking() {
    let dir = PathBuf::from(std::env::var("ED_JOURNAL_DIR").expect("set ED_JOURNAL_DIR"));
    let events = Reader::new(dir).poll();
    let mut state = State::default();
    events.iter().for_each(|e| state.apply(e));
    println!("{} events\n{}", events.len(), serde_json::to_string_pretty(&state).unwrap());
    assert!(!state.commander.name.is_empty());
}

#[test]
fn partial_lines_wait_for_their_newline() {
    let dir = std::env::temp_dir().join(format!("edw-partial-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("Journal.2026-10-11T120000.01.log");
    let mut file = std::fs::File::create(&path).unwrap();
    let mut reader = Reader::new(dir.clone());

    write!(file, "{{\"event\":\"Docked\",\"StationName\":\"A\"}}\n{{\"event\":\"Undo").unwrap();
    file.flush().unwrap();
    let first = reader.poll();
    assert_eq!(first.len(), 1, "only the complete line is returned");

    writeln!(file, "cked\"}}").unwrap();
    file.flush().unwrap();
    let second = reader.poll();
    assert_eq!(second.len(), 1);
    assert_eq!(second[0]["event"], "Undocked");

    assert!(reader.poll().is_empty(), "nothing is returned twice");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn new_journal_file_is_replayed_from_the_top() {
    let dir = std::env::temp_dir().join(format!("edw-rotate-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("Journal.2026-10-11T120000.01.log"), "{\"event\":\"Docked\"}\n").unwrap();
    let mut reader = Reader::new(dir.clone());
    assert_eq!(reader.poll().len(), 1);

    std::thread::sleep(std::time::Duration::from_millis(50));
    std::fs::write(dir.join("Journal.2026-10-11T130000.02.log"), "{\"event\":\"Continued\"}\n{\"event\":\"Undocked\"}\n").unwrap();
    assert_eq!(reader.poll().len(), 2);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn half_written_companion_file_is_retried() {
    let dir = std::env::temp_dir().join(format!("edw-companion-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut reader = Reader::new(dir.clone());

    std::fs::write(dir.join("Status.json"), "{\"event\":\"Status\",\"Fla").unwrap();
    assert!(reader.poll().is_empty());

    std::thread::sleep(std::time::Duration::from_millis(50));
    std::fs::write(dir.join("Status.json"), "{\"event\":\"Status\",\"Flags\":1}").unwrap();
    assert_eq!(reader.poll().len(), 1);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn second_commander_clears_the_first_ones_state() {
    let mut s = State::default();
    s.apply(&json!({"event":"Commander","FID":"F1","Name":"One"}));
    s.apply(&json!({"event":"Docked","StationName":"Somewhere","MarketID":1}));
    s.apply(&json!({"event":"Commander","FID":"F2","Name":"Two"}));
    assert_eq!(s.commander.name, "Two");
    assert!(!s.location.docked && s.location.station.is_none());
}

#[test]
fn loading_in_on_foot_keeps_the_last_ship() {
    let mut s = State::default();
    s.apply(&json!({"event":"Loadout","Ship":"sidewinder","ShipName":"","CargoCapacity":2}));
    s.apply(&json!({"event":"LoadGame","FID":"F1","Commander":"One","Ship":"FlightSuit","Credits":5}));
    assert_eq!(s.ship.model, "Sidewinder");
    assert_eq!(ed_wingman_lib::state::ship_model("some_new_ship"), "Some_new_ship");
}

#[test]
fn status_updates_the_hold_total_before_cargo_json_arrives() {
    let mut s = State::default();
    s.apply(&json!({"event":"Status","Flags":16777216,"Cargo":1.0,"Balance":49916}));
    assert_eq!((s.cargo.count, s.balance), (1, Some(49916)));
    // In the SRV the figure is the SRV's own hold, not the ship's.
    s.apply(&json!({"event":"Status","Flags":67108864,"Cargo":0.0,"Balance":49916}));
    assert_eq!(s.cargo.count, 1);
}

#[test]
fn game_closed_status_marks_not_running() {
    let mut s = State::default();
    s.apply(&json!({"event":"LoadGame","FID":"F1","Commander":"One","Credits":5}));
    assert!(s.running);
    s.apply(&json!({"timestamp":"t","event":"Status","Flags":0}));
    assert!(!s.running);
}

#[test]
fn beta_and_legacy_versions_are_flagged() {
    assert!(is_beta_or_legacy("4.1.0.100 Beta"));
    assert!(is_beta_or_legacy("3.8.0.407"));
    assert!(!is_beta_or_legacy("4.0.0.1904"));
}
