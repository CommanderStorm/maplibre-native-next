# @bartlwrs/mln-next-web

[maplibre-native-next](https://github.com/louwers/maplibre-native-next), a Rust
port of the [MapLibre Native](https://github.com/maplibre/maplibre-native) map
renderer, in the browser on WebGPU, behind a trimmed-down
[MapLibre GL JS](https://maplibre.org/maplibre-gl-js/docs/) API.

The renderer is compiled to WebAssembly and draws straight into a canvas with
WebGPU. With cross-origin isolation it runs in a worker, with tile parsing on
more workers, so the page's thread stays free.

This is experimental. The API is a subset of GL JS's `Map` and may change
between versions.

## Install

```sh
npm install @bartlwrs/mln-next-web
```

## Use

```js
import { Map } from "@bartlwrs/mln-next-web";

const map = new Map({
  container: "map", // an element or its id
  style: "https://tiles.openfreemap.org/styles/liberty",
  center: [4.9041, 52.3676], // [lng, lat]
  zoom: 12.5,
});

map.on("load", () => console.log("loaded"));
map.on("click", (event) => console.log(event.lngLat));
map.on("moveend", () => console.log(map.getCenter(), map.getZoom()));

map.flyTo({ center: [2.3522, 48.8566], zoom: 11 });
```

The container needs a size, as with GL JS. The map follows its size when it
changes.

## Browsers

The map needs [WebGPU](https://caniuse.com/webgpu). It has been tested in
Chromium (Chrome and Edge 113 and newer); other browsers with WebGPU should
work but haven't been tested. On Linux, Chromium enables WebGPU only with
Vulkan: turn on `chrome://flags/#enable-vulkan`.

## Threads and cross-origin isolation

When the page is
[cross-origin isolated](https://developer.mozilla.org/en-US/docs/Web/API/Window/crossOriginIsolated),
the map runs in a worker that owns the canvas, and its tile parsing and
layout run on more workers that share its memory. Serve the page with:

```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

Without them, the map runs on the page's thread and does its background work
between frames. That works anywhere, but large tiles can delay a frame. The
`threads` option overrides the choice.

## Bundlers and hosting

The package loads its worker and WebAssembly modules with
`new URL("…", import.meta.url)`, which Vite, webpack, Rollup and esbuild
understand. Serve them from the page's origin: browsers don't start module
workers from another origin, so loading the package from a CDN won't work.

The map loads two modules depending on cross-origin isolation, `wasm/single`
or `wasm/threads`. It downloads only the one it uses: about 1.6 MB with
brotli, 2.3 MB with gzip.

## API

A `Map` has these options, methods and events, which behave as GL JS's unless
noted.

**Options:** `container`, `style` (a URL or style JSON), `center`, `zoom`,
`bearing`, `pitch`, `minZoom`, `maxZoom`, `minPitch`, `maxPitch`,
`interactive`, `attributionControl` (`false` or `{customAttribution}`),
`pixelRatio`, and `threads`.

**Camera:**
- `jumpTo`, `easeTo` and `flyTo` (with `speed` and `curve`), plus `fitBounds` (with `padding`, `maxZoom`, `bearing`, `pitch`).
- `panBy`, `panTo`, `zoomTo`, `zoomIn`, `zoomOut`, `rotateTo`, `resetNorth`, `resetNorthPitch`.
- `setCenter`, `setZoom`, `setBearing`, `setPitch`, `stop`.
- `setMinZoom`, `setMaxZoom`, `setMinPitch`, `setMaxPitch`.

**State:**
- `getCenter`, `getZoom`, `getBearing`, `getPitch`, `getBounds`, and the min and max zoom and pitch.
- `loaded`, `isMoving`, `project`, `unproject`.
- `getContainer`, `getCanvasContainer`, `getCanvas`.

**Other:** `setStyle`, `resize`, `triggerRepaint`, `queryRenderedFeatures`, `remove`, and `on`/`once`/`off`.

**Events:**
- **Lifecycle:** `load`, `idle`, `render`, `styledata`, `styleimagemissing`, `error`, `resize`, `remove`.
- **Camera:** `movestart`, `move`, `moveend`, `zoomstart`, `zoom`, `zoomend`, `rotate`, `pitch`.
- **Mouse:** `mousedown`, `mouseup`, `mousemove`, `click`, `dblclick`, `contextmenu`, with `point`, `lngLat` and `originalEvent`.

**Interaction:** `dragPan`, `scrollZoom`, `doubleClickZoom`, `dragRotate`
(right- or control-drag), `touchZoomRotate` and `keyboard`, each with
`enable()`, `disable()` and `isEnabled()`.

Also exported: `LngLat`, `LngLatBounds` and `Point`.

### Differences from GL JS

- `queryRenderedFeatures` returns a promise, since the renderer may be in a
  worker. Its features are GeoJSON with `source` and `sourceLayer`, but no
  `layer` or `state`.
- `project`, `unproject` and the mouse events' `lngLat` use the camera of the
  last rendered frame. Right after a camera change, they lag by a frame.
- `fitBounds` centers the bounds; asymmetric `padding` sizes the view but
  doesn't offset it.
- There are no controls other than the attribution, and no markers, popups,
  terrain, or runtime style changes (`addLayer`, `setPaintProperty` and so on).
  Change the style with `setStyle`.
- Bearings are degrees in (-180, 180], as in GL JS. The core keeps them in
  [0, 360).

## Build

From a checkout of maplibre-native-next:

```sh
platform/web/build.sh   # writes platform/web/dist
```

It needs Rust with the `wasm32-unknown-unknown` target and the `rust-src`
component, and the `wasm-bindgen` CLI of the version `platform/web/Cargo.toml`
pins. `wasm-opt` is used when it's on `PATH`.

## License

BSD-2-Clause, as maplibre-native-next.
