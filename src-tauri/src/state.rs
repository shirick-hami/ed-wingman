//! Pure reducer: journal and companion-file events in, current game state out.
//! Unknown events and extra fields are ignored; keys and event names match case-insensitively.

use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub journal_dir: String,
    pub journal_found: bool,
    pub running: bool,
    pub game_version: String,
    pub odyssey: bool,
    /// Beta or Legacy (pre-4.0) client: nothing from it is uploaded or planned from.
    pub beta_or_legacy: bool,
    pub commander: Commander,
    pub location: Location,
    pub ship: Ship,
    pub balance: Option<i64>,
    pub cargo: Cargo,
    pub status: Status,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Commander {
    pub name: String,
    pub fid: String,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub system: String,
    /// The system's key; names can repeat or change case.
    pub system_address: Option<u64>,
    pub star_pos: Option<[f64; 3]>,
    pub body: Option<String>,
    pub station: Option<String>,
    pub market_id: Option<u64>,
    pub docked: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ship {
    #[serde(rename = "type")]
    pub kind: String,
    /// Model name for display, e.g. "Cobra Mk III" for `cobramkiii`.
    pub model: String,
    /// The name the player gave the ship; empty if they have not named it.
    pub name: String,
    pub ident: String,
    pub cargo_capacity: Option<u64>,
    pub max_jump_range: Option<f64>,
    pub rebuy: Option<i64>,
    pub hull_health: Option<f64>,
    pub fuel_capacity: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cargo {
    pub count: u64,
    pub items: Vec<CargoItem>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CargoItem {
    pub name: String,
    pub display: String,
    pub count: u64,
    pub stolen: u64,
    pub mission_id: Option<u64>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub flags: u32,
    pub flags2: u32,
    pub gui_focus: i64,
    pub legal_state: String,
    pub fuel_main: Option<f64>,
}

fn get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    let obj = v.as_object()?;
    obj.get(key).or_else(|| obj.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v))
}
fn s(v: &Value, k: &str) -> Option<String> {
    get(v, k)?.as_str().map(str::to_owned)
}
fn u(v: &Value, k: &str) -> Option<u64> {
    get(v, k)?.as_u64()
}
fn i(v: &Value, k: &str) -> Option<i64> {
    get(v, k)?.as_i64()
}
fn f(v: &Value, k: &str) -> Option<f64> {
    get(v, k)?.as_f64()
}
fn b(v: &Value, k: &str) -> bool {
    get(v, k).and_then(Value::as_bool).unwrap_or(false)
}

/// Beta builds say so in the version string; Legacy clients are major version 3 or lower.
pub fn is_beta_or_legacy(version: &str) -> bool {
    let major = version.split('.').next().and_then(|m| m.trim().parse::<u32>().ok());
    version.to_ascii_lowercase().contains("beta") || major.is_some_and(|m| m < 4)
}

/// Display name for the journal's internal ship id. The journal only localises it in some events.
/// Unknown ids (ships added after this table) fall back to the id with a capital first letter.
pub fn ship_model(kind: &str) -> String {
    let id = kind.to_ascii_lowercase();
    let known = match id.as_str() {
        "sidewinder" => "Sidewinder",
        "eagle" => "Eagle",
        "hauler" => "Hauler",
        "adder" => "Adder",
        "viper" => "Viper Mk III",
        "viper_mkiv" => "Viper Mk IV",
        "cobramkiii" => "Cobra Mk III",
        "cobramkiv" => "Cobra Mk IV",
        "cobramkv" => "Cobra Mk V",
        "type6" => "Type-6 Transporter",
        "type7" => "Type-7 Transporter",
        "type8" => "Type-8 Transporter",
        "type9" => "Type-9 Heavy",
        "type9_military" => "Type-10 Defender",
        "dolphin" => "Dolphin",
        "orca" => "Orca",
        "belugaliner" => "Beluga Liner",
        "asp" => "Asp Explorer",
        "asp_scout" => "Asp Scout",
        "diamondback" => "Diamondback Scout",
        "diamondbackxl" => "Diamondback Explorer",
        "vulture" => "Vulture",
        "python" => "Python",
        "python_nx" => "Python Mk II",
        "ferdelance" => "Fer-de-Lance",
        "mamba" => "Mamba",
        "anaconda" => "Anaconda",
        "krait_mkii" => "Krait Mk II",
        "krait_light" => "Krait Phantom",
        "typex" => "Alliance Chieftain",
        "typex_2" => "Alliance Crusader",
        "typex_3" => "Alliance Challenger",
        "independant_trader" => "Keelback",
        "mandalay" => "Mandalay",
        "corsair" => "Corsair",
        "empire_eagle" => "Imperial Eagle",
        "empire_courier" => "Imperial Courier",
        "empire_trader" => "Imperial Clipper",
        "cutter" => "Imperial Cutter",
        "federation_dropship" => "Federal Dropship",
        "federation_dropship_mkii" => "Federal Assault Ship",
        "federation_gunship" => "Federal Gunship",
        "federation_corvette" => "Federal Corvette",
        _ => "",
    };
    if !known.is_empty() {
        return known.to_owned();
    }
    let mut chars = kind.chars();
    chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

impl State {
    pub fn apply(&mut self, ev: &Value) {
        let Some(event) = s(ev, "event") else { return };
        match event.to_ascii_lowercase().as_str() {
            "fileheader" => {
                self.game_version = s(ev, "gameversion").unwrap_or_default();
                self.odyssey = b(ev, "Odyssey");
                self.beta_or_legacy = is_beta_or_legacy(&self.game_version);
            }
            "commander" | "loadgame" => {
                let fid = s(ev, "FID").unwrap_or_default();
                if !self.commander.fid.is_empty() && self.commander.fid != fid {
                    self.reset_commander();
                }
                self.commander.fid = fid;
                if let Some(name) = s(ev, "Name").or_else(|| s(ev, "Commander")) {
                    self.commander.name = name;
                }
                if event.eq_ignore_ascii_case("loadgame") {
                    self.running = true;
                    self.balance = i(ev, "Credits").or(self.balance);
                    // Loading in on foot reports the suit as the "ship"; keep the last real ship.
                    let kind = s(ev, "Ship").unwrap_or_default();
                    if !kind.to_ascii_lowercase().contains("suit") {
                        self.ship.model = ship_model(&kind);
                        self.ship.kind = kind;
                        self.ship.name = s(ev, "ShipName").unwrap_or_default();
                        self.ship.ident = s(ev, "ShipIdent").unwrap_or_default();
                        self.ship.fuel_capacity = f(ev, "FuelCapacity");
                    }
                }
            }
            "loadout" => {
                self.ship.kind = s(ev, "Ship").unwrap_or_default();
                self.ship.model = ship_model(&self.ship.kind);
                self.ship.name = s(ev, "ShipName").unwrap_or_default();
                self.ship.ident = s(ev, "ShipIdent").unwrap_or_default();
                self.ship.cargo_capacity = u(ev, "CargoCapacity");
                self.ship.max_jump_range = f(ev, "MaxJumpRange");
                self.ship.rebuy = i(ev, "Rebuy");
                self.ship.hull_health = f(ev, "HullHealth");
                self.ship.fuel_capacity = get(ev, "FuelCapacity").and_then(|fc| f(fc, "Main"));
            }
            "hulldamage" => {
                if !b(ev, "Fighter") && get(ev, "PlayerPilot").is_none_or(|p| p.as_bool() == Some(true)) {
                    self.ship.hull_health = f(ev, "Health");
                }
            }
            "repairall" => self.ship.hull_health = Some(1.0),
            "location" | "fsdjump" | "carrierjump" => {
                let loc = &mut self.location;
                loc.system = s(ev, "StarSystem").unwrap_or_default();
                loc.system_address = u(ev, "SystemAddress");
                loc.star_pos = get(ev, "StarPos").and_then(|p| {
                    let a = p.as_array()?;
                    Some([a.first()?.as_f64()?, a.get(1)?.as_f64()?, a.get(2)?.as_f64()?])
                });
                loc.body = s(ev, "Body");
                loc.docked = b(ev, "Docked");
                loc.station = if loc.docked { s(ev, "StationName") } else { None };
                loc.market_id = if loc.docked { u(ev, "MarketID") } else { None };
            }
            "supercruiseexit" | "approachbody" => self.location.body = s(ev, "Body"),
            "leavebody" | "supercruiseentry" => self.location.body = None,
            "docked" => {
                self.location.docked = true;
                self.location.station = s(ev, "StationName");
                self.location.market_id = u(ev, "MarketID");
            }
            "undocked" => {
                self.location.docked = false;
                self.location.station = None;
                self.location.market_id = None;
            }
            // Cargo.json is the truth; journal Cargo events without Inventory only say the file changed.
            "cargo" => {
                if let Some(inv) = get(ev, "Inventory").and_then(Value::as_array) {
                    self.cargo.items = inv
                        .iter()
                        .map(|it| {
                            let name = s(it, "Name").unwrap_or_default();
                            CargoItem {
                                display: s(it, "Name_Localised").unwrap_or_else(|| name.clone()),
                                name,
                                count: u(it, "Count").unwrap_or(0),
                                stolen: u(it, "Stolen").unwrap_or(0),
                                mission_id: u(it, "MissionID"),
                            }
                        })
                        .collect();
                    self.cargo.count = u(ev, "Count").unwrap_or_else(|| self.cargo.items.iter().map(|c| c.count).sum());
                }
            }
            "status" => {
                let flags = u(ev, "Flags").unwrap_or(0) as u32;
                self.status.flags = flags;
                self.status.flags2 = u(ev, "Flags2").unwrap_or(0) as u32;
                self.status.gui_focus = i(ev, "GuiFocus").unwrap_or(0);
                self.status.legal_state = s(ev, "LegalState").unwrap_or_default();
                self.status.fuel_main = get(ev, "Fuel").and_then(|fu| f(fu, "FuelMain"));
                if let Some(bal) = i(ev, "Balance") {
                    self.balance = Some(bal);
                }
                // With the game closed the file holds only timestamp, event and Flags: 0.
                let fields = ev.as_object().map_or(0, |o| o.len());
                self.running = !(flags == 0 && fields <= 3);
            }
            "shutdown" => self.running = false,
            _ => {}
        }
    }

    /// A different commander loaded: drop everything that belonged to the previous one.
    fn reset_commander(&mut self) {
        *self = State {
            journal_dir: std::mem::take(&mut self.journal_dir),
            journal_found: self.journal_found,
            running: self.running,
            game_version: std::mem::take(&mut self.game_version),
            odyssey: self.odyssey,
            beta_or_legacy: self.beta_or_legacy,
            ..State::default()
        };
    }
}
