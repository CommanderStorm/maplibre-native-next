// The map, its renderer and its file sources, on a canvas transferred from the
// page. Thread pool threads are more workers (mlnSpawnThread). Frames come
// from this worker's requestAnimationFrame.
import "./forward-console.js";
import init, { WebMap } from "./pkg/maplibre_native_next_web.js";

const STYLE_URL = "https://tiles.openfreemap.org/styles/liberty";

globalThis.mlnSpawnThread = (module, memory, entry, name) => {
  const worker = new Worker(new URL("./thread-worker.js", import.meta.url), { type: "module", name });
  worker.onmessage = ({ data }) => postMessage(data);
  worker.postMessage([module, memory, entry]);
};

const stats = {
  enabled: false, intervals: [], renders: [], frames: 0, last: 0, windowStart: performance.now(),
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
    const i = sorted(this.intervals), r = sorted(this.renders);
    const pick = (a, q) => a.length ? a[Math.min(a.length - 1, Math.floor(a.length * q))] : 0;
    if (i.length > 0 && this.enabled) {
      console.log(`FRAMES ${(this.frames * 1000 / (now - this.windowStart)).toFixed(0)} fps, ` +
        `interval p50 ${pick(i, 0.5).toFixed(1)} p95 ${pick(i, 0.95).toFixed(1)} max ${i[i.length - 1].toFixed(1)} ms, ` +
        `${i.filter((x) => x > 25).length} over 25 ms, render p50 ${pick(r, 0.5).toFixed(1)} max ${r[r.length - 1].toFixed(1)} ms`);
    }
    this.intervals = []; this.renders = []; this.frames = 0; this.windowStart = now;
  },
};

let map = null;
const pending = [];

function handle(data) {
  switch (data.type) {
    case "pan": map.pan(data.dx, data.dy); break;
    case "zoom": map.zoom(data.delta, data.x, data.y); break;
    case "resize": map.resize(data.width, data.height); break;
  }
}

async function start({ canvas, width, height, ratio, query }) {
  stats.enabled = new URLSearchParams(query).has("stats");
  await init();
  map = await WebMap.create_offscreen(canvas, width, height, ratio);
  map.set_style_url(STYLE_URL);
  map.jump_to(52.3676, 4.9041, 12.5);
  for (const data of pending.splice(0)) handle(data);
  postMessage({ status: "Loading the map…" });
  let loaded = false;
  function frame() {
    const t0 = performance.now();
    const drawn = map.render();
    const t1 = performance.now();
    stats.record(drawn, t0, t1);
    stats.report(t1);
    if (!loaded && drawn && map.is_fully_loaded()) {
      loaded = true;
      console.log("map loaded");
      postMessage({ status: "" });
    }
    requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);
}

onmessage = ({ data }) => {
  if (data.type === "start") {
    start(data).catch((error) => {
      console.error(error);
      postMessage({ status: "The map stopped: " + (error?.message ?? error), error: true });
    });
  } else if (map) {
    handle(data);
  } else {
    pending.push(data);
  }
};
