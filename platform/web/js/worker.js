// The threaded build's map worker: the engine, on the canvas the Map
// transferred here, with frames from this worker's requestAnimationFrame. The
// map's thread pools run on more workers (thread.js) that share the module
// and its memory.
import init, * as wasm from "./wasm/threads/maplibre_native_next_web.js";
import { runEngine } from "./engine.js";

// Called by the module (src/lib.rs, threads::spawn) for each pool thread.
globalThis.mlnSpawnThread = (module, memory, entry, name) => {
  const thread = new Worker(new URL("./thread.js", import.meta.url), { type: "module", name });
  thread.postMessage([module, memory, entry]);
};

let receive = null;
const pending = [];

self.onmessage = ({ data }) => {
  if (data.type === "init") {
    start(data);
  } else if (receive) {
    receive(data);
  } else {
    pending.push(data);
  }
};

async function start({ canvas, width, height, pixelRatio }) {
  try {
    await init();
    receive = await runEngine({
      wasm, canvas, width, height, pixelRatio,
      post: (message, transfer) => self.postMessage(message, transfer),
    });
    for (const data of pending.splice(0)) receive(data);
  } catch (error) {
    self.postMessage({ type: "failed", message: String(error?.message ?? error) });
  }
}
