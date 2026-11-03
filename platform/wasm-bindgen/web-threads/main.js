// The threaded variant of ../web: the map runs in a worker (map-worker.js) on
// a canvas transferred to it, and its thread pools run on more workers
// (thread-worker.js) that share the module's memory. This page only forwards
// input. Shared memory needs cross-origin isolation (COOP/COEP headers).
const canvas = document.getElementById("map");
const statusElement = document.getElementById("status");

function showStatus(text, isError) {
  statusElement.textContent = text;
  statusElement.className = isError ? "error" : "";
}

async function problem() {
  if (!window.crossOriginIsolated) return "This page needs cross-origin isolation (Cross-Origin-Opener-Policy and Cross-Origin-Embedder-Policy headers).";
  if (!navigator.gpu) return "WebGPU isn't available in this browser.";
  if (await navigator.gpu.requestAdapter()) return null;
  return "WebGPU found no GPU adapter.";
}

function start() {
  const worker = new Worker(new URL("./map-worker.js", import.meta.url), { type: "module", name: "map" });
  worker.onmessage = ({ data }) => {
    if (data.console) console[data.console](data.text);
    if (data.status !== undefined) showStatus(data.status, data.error);
  };
  worker.onerror = (event) => showStatus("The map stopped: " + event.message, true);
  const offscreen = canvas.transferControlToOffscreen();
  worker.postMessage({
    type: "start", canvas: offscreen, width: canvas.clientWidth, height: canvas.clientHeight,
    ratio: window.devicePixelRatio || 1, query: location.search,
  }, [offscreen]);

  const pan = (dx, dy) => worker.postMessage({ type: "pan", dx, dy });
  const zoom = (delta, x, y) => worker.postMessage({ type: "zoom", delta, x, y });
  window.demoPan = pan;
  window.demoZoom = zoom;
  let last = null;
  canvas.addEventListener("pointerdown", (event) => {
    canvas.setPointerCapture(event.pointerId);
    last = { x: event.clientX, y: event.clientY };
  });
  canvas.addEventListener("pointermove", (event) => {
    if (!last) return;
    pan(event.clientX - last.x, event.clientY - last.y);
    last = { x: event.clientX, y: event.clientY };
  });
  const end = () => { last = null; };
  canvas.addEventListener("pointerup", end);
  canvas.addEventListener("pointercancel", end);
  canvas.addEventListener("wheel", (event) => {
    event.preventDefault();
    const lines = event.deltaMode === WheelEvent.DOM_DELTA_LINE ? 40 : 1;
    zoom(-event.deltaY * lines / 400, event.offsetX, event.offsetY);
  }, { passive: false });
  canvas.addEventListener("dblclick", (event) => zoom(1, event.offsetX, event.offsetY));
  window.addEventListener("resize", () => worker.postMessage({ type: "resize", width: canvas.clientWidth, height: canvas.clientHeight }));
}

problem().then((message) => message ? showStatus(message, true) : start());
