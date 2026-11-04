// Runs an Engine (src/lib.rs) for a Map: in the map's worker (worker.js) in
// the threaded build, or on the page (the single-threaded build). The Map
// talks to it through messages either way:
//
//   to the engine:   {type: "call", name, args}, {type: "query", id, args},
//                    {type: "wheel", value, kind, x, y}, {type: "remove"}
//   from the engine: {type: "ready"}, {type: "failed", message},
//                    {type: "frame", drew, state, events, moving, attributions},
//                    {type: "query", id, features}
//
// Each animation frame runs the engine's frame(): its RunLoop tasks, then a
// draw when the map changed. Without threads, the background work (tile
// parsing and layout) runs right after, in a posted task with a time budget,
// so that the frame is presented first. Scroll zoom steps (scroll_zoom.js)
// apply right before each frame draws.

import { ScrollZoom } from "./scroll_zoom.js";

const WORK_BUDGET_MS = 8;

const nextFrame = globalThis.requestAnimationFrame
  ? (callback) => globalThis.requestAnimationFrame(callback)
  : (callback) => setTimeout(callback, 16);

/**
 * Starts an engine on `canvas` (an HTMLCanvasElement or OffscreenCanvas).
 * `wasm` is the initialized wasm-bindgen module; `post(message, transfer)`
 * reaches the Map. Returns the function that takes the Map's messages.
 */
export async function runEngine({ wasm, canvas, width, height, pixelRatio, post }) {
  const engine = await wasm.Engine.create(canvas, width, height, pixelRatio);
  const threaded = wasm.threaded();
  const scrollZoom = new ScrollZoom(engine);
  let removed = false;
  let moving = false;
  let attributions = "";
  let workScheduled = false;

  // Posting to a MessageChannel runs the work as a task of its own, after
  // the frame's task (and its presentation).
  const workChannel = threaded ? null : new MessageChannel();
  if (workChannel) {
    workChannel.port1.onmessage = () => {
      workScheduled = false;
      if (!removed) engine.work(WORK_BUDGET_MS);
    };
  }

  function frame() {
    if (removed) return;
    scrollZoom.frame();
    const drew = engine.frame();
    const state = engine.state();
    const events = engine.take_events();
    const nowMoving = engine.is_moving();
    let changedAttributions = null;
    if (drew || events.length > 0) {
      const current = engine.attributions();
      const joined = current.join("\n");
      if (joined !== attributions) {
        attributions = joined;
        changedAttributions = current;
      }
    }
    if (drew || state.length > 0 || events.length > 0 || nowMoving !== moving || changedAttributions) {
      moving = nowMoving;
      post(
        { type: "frame", drew, state: state.length > 0 ? state : null, events, moving, attributions: changedAttributions },
        state.length > 0 ? [state.buffer] : [],
      );
    }
    if (workChannel && !workScheduled && engine.has_work()) {
      workScheduled = true;
      workChannel.port2.postMessage(null);
    }
    nextFrame(frame);
  }

  post({ type: "ready" });
  nextFrame(frame);

  return function receive(message) {
    if (removed) return;
    switch (message.type) {
      case "call":
        if (message.name === "set_limits") [scrollZoom.minZoom, scrollZoom.maxZoom] = message.args;
        try {
          engine[message.name](...message.args);
        } catch (error) {
          post({ type: "error", message: String(error?.message ?? error) });
        }
        break;
      case "query": {
        const [x0, y0, x1, y1, layers] = message.args;
        let features = [];
        try {
          features = JSON.parse(engine.query_rendered_features(x0, y0, x1, y1, layers));
        } catch (error) {
          post({ type: "error", message: String(error?.message ?? error) });
        }
        post({ type: "query", id: message.id, features });
        break;
      }
      case "wheel":
        scrollZoom.wheel(message);
        break;
      case "remove":
        removed = true;
        engine.free();
        break;
    }
  };
}
