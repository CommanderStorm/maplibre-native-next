// One thread of the map's thread pools (threaded build): runs the closure the
// spawning thread handed over (mln_thread_entry) on the shared module and
// memory.
import init, { mln_thread_entry } from "./wasm/threads/maplibre_native_next_web.js";

self.onmessage = async ({ data: [module, memory, entry] }) => {
  await init({ module_or_path: module, memory });
  mln_thread_entry(entry);
  close();
};
