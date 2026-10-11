const { listen } = window.__TAURI__.event;
const { invoke } = window.__TAURI__.core;

const $ = (id) => document.getElementById(id);
const credits = new Intl.NumberFormat("en-GB");

function notice(s) {
  if (!s.journalFound) return `No journal folder at ${s.journalDir || "the usual Saved Games location"}. Set it in settings.`;
  if (s.betaOrLegacy) return `Beta or Legacy client (${s.gameVersion}). Nothing from this session is shared or planned from.`;
  if (!s.running) return "Waiting for the game to start.";
  return "";
}

function render(s) {
  const n = notice(s);
  $("notice").textContent = n;
  $("notice").hidden = !n;

  const loc = s.location;
  $("cmdr").textContent = s.commander.name;
  $("system").textContent = loc.system || "Unknown system";
  $("place").textContent = loc.docked ? `Docked at ${loc.station}` : loc.body ? `Near ${loc.body}` : "In flight";
  $("ship").textContent = s.ship.name || s.ship.type || "-";
  $("credits").textContent = s.balance == null ? "-" : `${credits.format(s.balance)} CR`;

  const cap = s.ship.cargoCapacity;
  $("cargo").textContent = cap == null ? `${s.cargo.count} t` : `${s.cargo.count} / ${cap} t`;
  $("cargo-fill").style.width = cap ? `${Math.min(100, (s.cargo.count / cap) * 100)}%` : "0";
}

listen("state", (e) => render(e.payload));
listen("layout", (e) => document.body.classList.toggle("layout", e.payload));

// data-tauri-drag-region only reacts to the element itself, not its children, so drag from anywhere here.
document.addEventListener("mousedown", (e) => {
  if (e.button === 0 && document.body.classList.contains("layout")) {
    window.__TAURI__.window.getCurrentWindow().startDragging();
  }
});

invoke("get_state").then(render);
