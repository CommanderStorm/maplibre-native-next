// OpenFreeMap's Liberty style rendered by maplibre-native-next, the Rust port
// of MapLibre Native, built for wasm32-unknown-unknown with wasm-bindgen: no
// Emscripten, no threads, so no cross-origin isolation either.
//
// Each animation frame calls map.render(), which draws straight into the
// canvas when the map changed. The background work (tile parsing and layout)
// runs right after the frame, in a posted message, for a time budget, so a
// frame is presented before the work that follows it.
//
// URL parameters: ?budget=<ms> (background work per frame, default 8),
// ?stats (logs frame statistics every 2 s).
import init, { WebMap } from "./pkg/maplibre_native_next_web.js";

const STYLE_URL = "https://tiles.openfreemap.org/styles/liberty";
const canvas = document.getElementById("map");
const statusElement = document.getElementById("status");
const params = new URLSearchParams(location.search);
const budget = Number(params.get("budget") ?? 8);

function showStatus(text, isError) {
  statusElement.textContent = text;
  statusElement.className = isError ? "error" : "";
}

// navigator.gpu can exist without an adapter: Chromium on Linux renders
// WebGPU through Vulkan, which it leaves off unless chrome://flags/#enable-vulkan
// (or --enable-features=Vulkan) turns it on.
async function problem() {
  if (!navigator.gpu) return "WebGPU isn't available in this browser.";
  if (await navigator.gpu.requestAdapter()) return null;
  const linux = /Linux/.test(navigator.userAgent) && !/Android/.test(navigator.userAgent);
  return "WebGPU found no GPU adapter." + (linux
    ? " On Linux, Chromium needs Vulkan: enable chrome://flags/#enable-vulkan and restart the browser."
    : "");
}

// Frame statistics, as the Emscripten demo's timing build prints them.
const stats = {
  enabled: params.has("stats"),
  intervals: [], renders: [], works: [], frames: 0, last: 0, windowStart: performance.now(),
  record(drawn, t0, t1) {
    if (!drawn) return;
    if (this.last > 0) this.intervals.push(t1 - this.last);
    this.last = t1;
    this.renders.push(t1 - t0);
    this.frames++;
  },
  report(now) {
    if (now - this.windowStart < 2000) return;
    const sorted = (a) => a.slice().sort((x, y) => x - y);
    const i = sorted(this.intervals), r = sorted(this.renders), w = sorted(this.works);
    const pick = (a, q) => a.length ? a[Math.min(a.length - 1, Math.floor(a.length * q))] : 0;
    if (i.length > 0 && this.enabled) {
      console.log(`FRAMES ${(this.frames * 1000 / (now - this.windowStart)).toFixed(0)} fps, ` +
        `interval p50 ${pick(i, 0.5).toFixed(1)} p95 ${pick(i, 0.95).toFixed(1)} max ${i[i.length - 1].toFixed(1)} ms, ` +
        `${i.filter((x) => x > 25).length} over 25 ms, render p50 ${pick(r, 0.5).toFixed(1)} max ${r[r.length - 1].toFixed(1)} ms, ` +
        `work p50 ${pick(w, 0.5).toFixed(1)} max ${w.length ? w[w.length - 1].toFixed(1) : 0} ms (${w.length})`);
    }
    this.intervals = []; this.renders = []; this.works = []; this.frames = 0; this.windowStart = now;
  },
};

async function start() {
  await init();
  const ratio = window.devicePixelRatio || 1;
  const map = await WebMap.create(canvas, canvas.clientWidth, canvas.clientHeight, ratio);
  map.set_style_url(STYLE_URL);
  map.jump_to(52.3676, 4.9041, 12.5);
  window.map = map;

  // Background work right after each frame.
  const channel = new MessageChannel();
  let workPosted = false;
  channel.port1.onmessage = () => {
    workPosted = false;
    const t0 = performance.now();
    map.work(budget);
    stats.works.push(performance.now() - t0);
  };

  let loaded = false;
  function frame() {
    const t0 = performance.now();
    const drawn = map.render();
    const t1 = performance.now();
    stats.record(drawn, t0, t1);
    stats.report(t1);
    if (!loaded && drawn && map.is_fully_loaded()) {
      loaded = true;
      showStatus("", false);
    }
    if (!workPosted && map.has_work()) {
      workPosted = true;
      channel.port2.postMessage(0);
    }
    requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);
  showStatus("Loading the map…", false);

  // Input, also for scripted tests (window.demoPan / window.demoZoom).
  window.demoPan = (dx, dy) => map.pan(dx, dy);
  window.demoZoom = (delta, x, y) => map.zoom(delta, x, y);
  let last = null;
  canvas.addEventListener("pointerdown", (event) => {
    canvas.setPointerCapture(event.pointerId);
    last = { x: event.clientX, y: event.clientY };
  });
  canvas.addEventListener("pointermove", (event) => {
    if (!last) return;
    map.pan(event.clientX - last.x, event.clientY - last.y);
    last = { x: event.clientX, y: event.clientY };
  });
  const end = () => { last = null; };
  canvas.addEventListener("pointerup", end);
  canvas.addEventListener("pointercancel", end);
  canvas.addEventListener("wheel", (event) => {
    event.preventDefault();
    const lines = event.deltaMode === WheelEvent.DOM_DELTA_LINE ? 40 : 1;
    map.zoom(-event.deltaY * lines / 400, event.offsetX, event.offsetY);
  }, { passive: false });
  canvas.addEventListener("dblclick", (event) => map.zoom(1, event.offsetX, event.offsetY));
  window.addEventListener("resize", () => map.resize(canvas.clientWidth, canvas.clientHeight));
}

problem().then((message) => {
  if (message) { showStatus(message, true); return; }
  start().catch((error) => {
    console.error(error);
    showStatus("The map stopped: " + (error?.message ?? error), true);
  });
});
