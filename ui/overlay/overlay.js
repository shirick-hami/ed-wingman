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
const { getCurrentWindow, LogicalSize } = window.__TAURI__.window;
const win = getCurrentWindow();

document.addEventListener("mousedown", (e) => {
  if (e.button !== 0 || !document.body.classList.contains("layout")) return;
  // Only the width is dragged: text scales with width and the height follows the content below.
  if (e.target.id === "grip") win.startResizeDragging("East");
  else win.startDragging();
});

// Fit the window height to the panel so there is never empty space under the content.
const MARGIN = 12; // #panel's 6px margin, top and bottom
new ResizeObserver(() => {
  const h = Math.ceil($("panel").getBoundingClientRect().height) + MARGIN;
  if (Math.abs(window.innerHeight - h) > 1) win.setSize(new LogicalSize(window.innerWidth, h));
}).observe($("panel"));

invoke("get_state").then(render);
