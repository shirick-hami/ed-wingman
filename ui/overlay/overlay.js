const { listen } = window.__TAURI__.event;
const { invoke } = window.__TAURI__.core;

const $ = (id) => document.getElementById(id);
const credits = new Intl.NumberFormat("en-GB");

function notice(s) {
  if (!s.journalFound) return `Journal folder not found: ${s.journalDir || "(unknown)"}`;
  if (s.betaOrLegacy) return `Beta or Legacy client (${s.gameVersion}): data is not shared or planned from.`;
  if (!s.running) return "Game not running.";
  return "";
}

function render(s) {
  const n = notice(s);
  $("notice").textContent = n;
  $("notice").hidden = !n;

  const loc = s.location;
  $("cmdr").textContent = s.commander.name || "-";
  $("system").textContent = [loc.system, loc.body].filter(Boolean).join(" · ") || "-";
  $("station").textContent = loc.docked ? loc.station : "-";
  $("ship").textContent = [s.ship.name || s.ship.type, s.ship.ident].filter(Boolean).join(" ") || "-";
  $("credits").textContent = s.balance == null ? "-" : `${credits.format(s.balance)} CR`;
  $("cargo").textContent = s.ship.cargoCapacity == null ? `${s.cargo.count} t` : `${s.cargo.count} / ${s.ship.cargoCapacity} t`;
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
