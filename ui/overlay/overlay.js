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
  $("ship").textContent = s.ship.name || s.ship.model || "-";
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
  if (e.button === 0 && e.target.id !== "grip" && document.body.classList.contains("layout")) win.startDragging();
});

const MARGIN = 12; // #panel's 6px margin, top and bottom
const MIN_WIDTH = 260;
const panelHeight = () => Math.ceil($("panel").getBoundingClientRect().height) + MARGIN;

// Fit the window height to the panel so there is never empty space under the content.
let resizing = null;
new ResizeObserver(() => {
  if (!resizing && Math.abs(window.innerHeight - panelHeight()) > 1) {
    win.setSize(new LogicalSize(window.innerWidth, panelHeight()));
  }
}).observe($("panel"));

// Resizing is done here rather than by Windows: the content is laid out at the target width first,
// then width and height are set together in one call per frame. Letting Windows resize while this
// page corrected the height made the two fight, which is what caused the blinking and clipping.
function layoutAt(width) {
  document.documentElement.style.fontSize = `${Math.min(24, Math.max(10, width * 0.036))}px`;
  document.body.style.width = `${width}px`;
  win.setSize(new LogicalSize(width, panelHeight()));
}

const grip = $("grip");
grip.addEventListener("pointerdown", (e) => {
  e.preventDefault();
  grip.setPointerCapture(e.pointerId);
  resizing = { x0: e.screenX, w0: window.innerWidth, frame: 0 };
});
grip.addEventListener("pointermove", (e) => {
  if (!resizing) return;
  const width = Math.max(MIN_WIDTH, Math.round(resizing.w0 + e.screenX - resizing.x0));
  cancelAnimationFrame(resizing.frame);
  resizing.frame = requestAnimationFrame(() => layoutAt(width));
});
grip.addEventListener("pointerup", () => {
  resizing = null;
  // The window now matches the chosen width, so the width-based sizes in the stylesheet take over again.
  document.documentElement.style.fontSize = "";
  document.body.style.width = "";
});

invoke("get_state").then(render);
