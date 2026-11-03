// The Map: a trimmed-down version of MapLibre GL JS's Map API on
// maplibre-native-next. See ../README.md for what it has and how it differs.
//
// The engine (engine.js, src/lib.rs) renders straight into the map's canvas.
// With cross-origin isolation (SharedArrayBuffer) it runs in a worker that
// owns the canvas, with its thread pools on more workers; without, it runs
// on this thread and does its background work between frames. Either way the
// Map talks to it through messages and keeps a copy of the camera and
// projection the engine last reported, so that getters, project() and
// unproject() answer synchronously.

import { AttributionControl } from "./attribution.js";
import { Evented } from "./evented.js";
import { LngLat, LngLatBounds, Point, Projection } from "./geo.js";
import { Handlers } from "./handlers.js";

const DEFAULT_DURATION = 500;

const CSS = `
.mln-map { overflow: hidden; }
.mln-view { position: absolute; inset: 0; overflow: hidden; touch-action: none; cursor: grab; outline: none; }
.mln-view.mln-grabbing { cursor: grabbing; }
/* One canvas pixel per device pixel, centered, at any size: the box is the
   view in device pixels, scaled back down, and object-fit: none never scales
   the frame. A resize changes the box before the engine's frame at the new
   size arrives; meanwhile the previous frame keeps its size and position
   instead of stretching. */
.mln-canvas { position: absolute; left: 50%; top: 50%;
  width: calc(100% * var(--mln-pixel-ratio, 1)); height: calc(100% * var(--mln-pixel-ratio, 1));
  transform: translate(-50%, -50%) scale(calc(1 / var(--mln-pixel-ratio, 1)));
  object-fit: none; pointer-events: none; }
.mln-attribution { position: absolute; right: 0; bottom: 0; padding: 2px 6px; font: 12px/20px system-ui, sans-serif;
  color: rgba(0, 0, 0, 0.75); background: rgba(255, 255, 255, 0.7); }
.mln-attribution a { color: inherit; }
`;

function injectStyle() {
  if (document.getElementById("mln-next-web-style")) return;
  const style = document.createElement("style");
  style.id = "mln-next-web-style";
  style.textContent = CSS;
  document.head.appendChild(style);
}

// NaN tells the engine to leave a camera value as it is.
const or = (value) => (value === undefined || value === null ? NaN : Number(value));

export class Map extends Evented {
  /**
   * @param {object} options
   * @param {HTMLElement|string} options.container The element, or its id.
   * @param {string|object} [options.style] A style URL or style JSON.
   * @param {LngLatLike} [options.center=[0, 0]]
   * @param {number} [options.zoom=0]
   * @param {number} [options.bearing=0]
   * @param {number} [options.pitch=0]
   * @param {number} [options.minZoom] @param {number} [options.maxZoom]
   * @param {number} [options.minPitch] @param {number} [options.maxPitch]
   * @param {boolean} [options.interactive=true]
   * @param {false|{customAttribution?: string|string[]}} [options.attributionControl]
   * @param {number} [options.pixelRatio=devicePixelRatio]
   * @param {boolean} [options.threads] Run in workers; the default is to when
   *   the page is cross-origin isolated.
   */
  constructor(options) {
    super();
    const container = typeof options?.container === "string" ? document.getElementById(options.container) : options?.container;
    if (!(container instanceof HTMLElement)) throw new Error(`Container '${options?.container}' not found.`);
    injectStyle();
    this._container = container;
    container.classList.add("mln-map");
    if (getComputedStyle(container).position === "static") container.style.position = "relative";

    this._view = document.createElement("div");
    this._view.className = "mln-view";
    this._view.tabIndex = 0;
    this._view.setAttribute("aria-label", "Map");
    this._canvas = document.createElement("canvas");
    this._canvas.className = "mln-canvas";
    this._view.appendChild(this._canvas);
    container.appendChild(this._view);

    this._pixelRatio = options.pixelRatio ?? globalThis.devicePixelRatio ?? 1;
    this._view.style.setProperty("--mln-pixel-ratio", String(this._pixelRatio));
    this._width = Math.max(1, this._view.clientWidth);
    this._height = Math.max(1, this._view.clientHeight);
    this._camera = {
      center: LngLat.convert(options.center ?? [0, 0]),
      zoom: options.zoom ?? 0,
      bearing: options.bearing ?? 0,
      pitch: options.pitch ?? 0,
    };
    this._limits = {
      minZoom: options.minZoom ?? 0,
      maxZoom: options.maxZoom ?? 22,
      minPitch: options.minPitch ?? 0,
      maxPitch: options.maxPitch ?? 60,
    };
    this._projection = null;
    this._loaded = false;
    this._moving = false;
    this._zooming = false;
    this._removed = false;
    this._queue = [];
    this._send = null;
    this._queries = new globalThis.Map();
    this._nextQuery = 1;

    const handlers = new Handlers(this, this._view, options.interactive !== false);
    this._handlers = handlers;
    this.dragPan = handlers.dragPan;
    this.scrollZoom = handlers.scrollZoom;
    this.doubleClickZoom = handlers.doubleClickZoom;
    this.dragRotate = handlers.dragRotate;
    this.touchZoomRotate = handlers.touchZoomRotate;
    this.keyboard = handlers.keyboard;

    if (options.attributionControl !== false) {
      this._attribution = new AttributionControl(this, options.attributionControl ?? {});
    }

    this._resizeObserver = new ResizeObserver(() => this.resize());
    this._resizeObserver.observe(this._view);

    this._call("set_limits", this._limits.minZoom, this._limits.maxZoom, this._limits.minPitch, this._limits.maxPitch);
    const { center, zoom, bearing, pitch } = this._camera;
    this._call("jump_to", center.lng, center.lat, zoom, bearing, pitch);
    if (options.style) this.setStyle(options.style);

    const threaded = options.threads ?? globalThis.crossOriginIsolated === true;
    this._start(threaded).catch((error) => this._fail(error?.message ?? String(error)));
  }

  // MARK: - Engine

  async _start(threaded) {
    const init = { width: this._width, height: this._height, pixelRatio: this._pixelRatio };
    if (threaded) {
      const worker = new Worker(new URL("./worker.js", import.meta.url), { type: "module", name: "mln-next-web" });
      worker.onmessage = ({ data }) => this._receive(data);
      worker.onerror = (event) => this._fail(event.message ?? "the map worker failed");
      const canvas = this._canvas.transferControlToOffscreen();
      worker.postMessage({ type: "init", canvas, ...init }, [canvas]);
      this._worker = worker;
      // The worker keeps messages until its engine is up.
      this._connect((message) => worker.postMessage(message));
      return;
    }
    const [{ runEngine }, wasm] = await Promise.all([
      import("./engine.js"),
      import("./wasm/single/maplibre_native_next_web.js"),
    ]);
    await wasm.default();
    if (this._removed) return;
    const receive = await runEngine({ wasm, canvas: this._canvas, ...init, post: (message) => this._receive(message) });
    this._connect(receive);
  }

  _connect(send) {
    this._send = send;
    for (const message of this._queue.splice(0)) send(message);
  }

  _post(message) {
    if (this._removed) return;
    if (this._send) this._send(message);
    else this._queue.push(message);
  }

  /** Calls an Engine method (src/lib.rs) by name. */
  _call(name, ...args) {
    this._post({ type: "call", name, args });
  }

  _fail(message) {
    this.fire("error", { error: new Error(message) });
  }

  _receive(message) {
    if (this._removed) return;
    switch (message.type) {
      case "frame":
        this._frame(message);
        break;
      case "query": {
        const resolve = this._queries.get(message.id);
        this._queries.delete(message.id);
        resolve?.(
          message.features.map(({ feature, source, sourceLayer }) => ({ ...feature, source, sourceLayer })),
        );
        break;
      }
      case "error":
      case "failed":
        this._fail(message.message);
        break;
    }
  }

  _frame({ drew, state, events, moving, attributions }) {
    let moved = false;
    let zoomed = false;
    let rotated = false;
    let pitched = false;
    if (state) {
      const before = this._projection;
      const now = new Projection(state);
      this._projection = now;
      this._camera = { center: now.center, zoom: now.zoom, bearing: now.bearing, pitch: now.pitch };
      if (before) {
        zoomed = now.zoom !== before.zoom;
        rotated = now.bearing !== before.bearing;
        pitched = now.pitch !== before.pitch;
        moved = zoomed || rotated || pitched || now.center.lng !== before.center.lng || now.center.lat !== before.center.lat;
      }
    }
    if (moved) {
      if (!this._moving) {
        this._moving = true;
        this.fire("movestart");
      }
      if (zoomed && !this._zooming) {
        this._zooming = true;
        this.fire("zoomstart");
      }
      this.fire("move");
      if (zoomed) this.fire("zoom");
      if (rotated) this.fire("rotate");
      if (pitched) this.fire("pitch");
    }
    if (attributions) this._attribution?.setSourceAttributions(attributions);
    for (const [type, detail] of events) {
      switch (type) {
        case "load":
          this._loaded = true;
          this.fire("load");
          break;
        case "styledata":
          this.fire("styledata", { dataType: "style" });
          break;
        case "styleimagemissing":
          this.fire("styleimagemissing", { id: detail });
          break;
        case "error":
          this._fail(detail);
          break;
        default:
          this.fire(type);
      }
    }
    if (drew) this.fire("render");
    if (this._moving && !moving) {
      this._moving = false;
      if (this._zooming) {
        this._zooming = false;
        this.fire("zoomend");
      }
      this.fire("moveend");
    }
  }

  _size() {
    return { width: this._width, height: this._height };
  }

  _fireMouse(type, originalEvent) {
    if (!this.listens(type)) return;
    const rect = this._view.getBoundingClientRect();
    const point = new Point(originalEvent.clientX - rect.left, originalEvent.clientY - rect.top);
    this.fire(type, { point, lngLat: this.unproject(point), originalEvent });
  }

  // MARK: - State

  getContainer() { return this._container; }
  getCanvasContainer() { return this._view; }
  getCanvas() { return this._canvas; }
  getCenter() { return this._camera.center; }
  getZoom() { return this._camera.zoom; }
  getBearing() { return this._camera.bearing; }
  getPitch() { return this._camera.pitch; }
  getMinZoom() { return this._limits.minZoom; }
  getMaxZoom() { return this._limits.maxZoom; }
  getMinPitch() { return this._limits.minPitch; }
  getMaxPitch() { return this._limits.maxPitch; }

  /** Whether the style and the tiles in view have loaded once ('load' fired). */
  loaded() { return this._loaded; }

  /** Whether the camera is moving: a gesture or an animation. */
  isMoving() { return this._moving; }

  /** The bounds of the area in view (of the last frame). */
  getBounds() {
    const { _width: w, _height: h } = this;
    const bounds = new LngLatBounds();
    for (const point of [new Point(0, 0), new Point(w, 0), new Point(w, h), new Point(0, h)]) {
      bounds.extend(this.unproject(point));
    }
    return bounds;
  }

  /** The pixel (relative to the container) of a LngLatLike, for the last frame. */
  project(lngLat) {
    const position = LngLat.convert(lngLat);
    if (!this._projection) return new Point(this._width / 2, this._height / 2);
    return this._projection.project(position);
  }

  /** The LngLat at a pixel (relative to the container), for the last frame. */
  unproject(point) {
    const p = Point.convert(point);
    if (!this._projection) return this._camera.center;
    return this._projection.unproject(p);
  }

  // MARK: - Camera

  jumpTo(options = {}) {
    const camera = this._apply(options);
    this._call("jump_to", camera.lng, camera.lat, camera.zoom, camera.bearing, camera.pitch);
    return this;
  }

  easeTo(options = {}) {
    if (options.animate === false || options.duration === 0) return this.jumpTo(options);
    const camera = this._apply(options);
    this._call("ease_to", camera.lng, camera.lat, camera.zoom, camera.bearing, camera.pitch, options.duration ?? DEFAULT_DURATION);
    return this;
  }

  /** Flies along a zoom-out-and-in path. `speed` and `curve` as GL JS's. */
  flyTo(options = {}) {
    if (options.animate === false) return this.jumpTo(options);
    const camera = this._apply(options);
    const velocity = options.speed !== undefined ? options.speed * (options.curve ?? 1.42) : NaN;
    this._call("fly_to", camera.lng, camera.lat, camera.zoom, camera.bearing, camera.pitch, or(options.duration), velocity);
    return this;
  }

  /** Pans by `offset` pixels: [x, y] moves the view right and down. */
  panBy(offset, options = {}) {
    const p = Point.convert(offset);
    this._call("move_by", -p.x, -p.y, options.animate === false ? NaN : (options.duration ?? DEFAULT_DURATION));
    return this;
  }

  panTo(lngLat, options = {}) {
    return this.easeTo({ ...options, center: lngLat });
  }

  zoomTo(zoom, options = {}) {
    return this.easeTo({ ...options, zoom });
  }

  zoomIn(options = {}) {
    return this.zoomTo(this.getZoom() + 1, options);
  }

  zoomOut(options = {}) {
    return this.zoomTo(this.getZoom() - 1, options);
  }

  rotateTo(bearing, options = {}) {
    return this.easeTo({ ...options, bearing });
  }

  resetNorth(options = {}) {
    return this.rotateTo(0, options);
  }

  resetNorthPitch(options = {}) {
    return this.easeTo({ ...options, bearing: 0, pitch: 0 });
  }

  setCenter(center) { return this.jumpTo({ center }); }
  setZoom(zoom) { return this.jumpTo({ zoom }); }
  setBearing(bearing) { return this.jumpTo({ bearing }); }
  setPitch(pitch) { return this.jumpTo({ pitch }); }

  /**
   * Shows `bounds` (LngLatBoundsLike), inside `padding` pixels (a number or
   * {top, bottom, left, right}).
   */
  fitBounds(bounds, options = {}) {
    const b = LngLatBounds.convert(bounds);
    const p = typeof options.padding === "number"
      ? { top: options.padding, bottom: options.padding, left: options.padding, right: options.padding }
      : { top: 0, bottom: 0, left: 0, right: 0, ...options.padding };
    const duration = options.animate === false ? NaN : (options.duration ?? DEFAULT_DURATION);
    this._call(
      "fit_bounds", b.getWest(), b.getSouth(), b.getEast(), b.getNorth(),
      p.top, p.left, p.bottom, p.right, or(options.bearing), or(options.pitch), or(options.maxZoom), duration,
    );
    return this;
  }

  /** Stops any camera animation. */
  stop() {
    this._call("cancel_transitions");
    return this;
  }

  setMinZoom(zoom) { return this._setLimits({ minZoom: zoom ?? 0 }); }
  setMaxZoom(zoom) { return this._setLimits({ maxZoom: zoom ?? 22 }); }
  setMinPitch(pitch) { return this._setLimits({ minPitch: pitch ?? 0 }); }
  setMaxPitch(pitch) { return this._setLimits({ maxPitch: pitch ?? 60 }); }

  _setLimits(limits) {
    Object.assign(this._limits, limits);
    const l = this._limits;
    this._call("set_limits", l.minZoom, l.maxZoom, l.minPitch, l.maxPitch);
    return this;
  }

  // The camera an options object asks for, as engine arguments, with the
  // getters updated at once as GL JS updates them.
  _apply(options) {
    const center = options.center !== undefined ? LngLat.convert(options.center) : null;
    if (center) this._camera.center = center;
    for (const key of ["zoom", "bearing", "pitch"]) {
      if (options[key] !== undefined) this._camera[key] = Number(options[key]);
    }
    return {
      lng: center ? center.lng : NaN,
      lat: center ? center.lat : NaN,
      zoom: or(options.zoom),
      bearing: or(options.bearing),
      pitch: or(options.pitch),
    };
  }

  // MARK: - Style, size, queries, lifecycle

  /** Loads a style from a URL or from its JSON (an object). */
  setStyle(style) {
    if (typeof style === "string") this._call("set_style_url", new URL(style, location.href).href);
    else this._call("set_style_json", JSON.stringify(style));
    return this;
  }

  /** Follows the container's size; the map also does when it changes. */
  resize() {
    const width = Math.max(1, this._view.clientWidth);
    const height = Math.max(1, this._view.clientHeight);
    if (width === this._width && height === this._height) return this;
    this._width = width;
    this._height = height;
    this._call("resize", width, height);
    this.fire("resize");
    return this;
  }

  triggerRepaint() {
    this._call("trigger_repaint");
  }

  /**
   * The rendered features at a point or in a box ([[x0, y0], [x1, y1]]), or
   * in view when there is no geometry, as GeoJSON features with `source` and
   * `sourceLayer`. Unlike GL JS's, this returns a promise: the renderer may
   * be in a worker.
   */
  queryRenderedFeatures(geometry, options = {}) {
    if (geometry && !Array.isArray(geometry) && !(geometry instanceof Point) && !("x" in geometry)) {
      options = geometry;
      geometry = undefined;
    }
    let args;
    if (geometry === undefined) {
      args = [0, 0, this._width, this._height];
    } else if (Array.isArray(geometry) && Array.isArray(geometry[0])) {
      const [a, b] = geometry.map(Point.convert);
      args = [a.x, a.y, b.x, b.y];
    } else {
      const p = Point.convert(geometry);
      args = [p.x, p.y, NaN, NaN];
    }
    const id = this._nextQuery++;
    return new Promise((resolve) => {
      this._queries.set(id, resolve);
      this._post({ type: "query", id, args: [...args, options.layers ?? []] });
    });
  }

  /** Destroys the map: its engine, workers, DOM and listeners. */
  remove() {
    if (this._removed) return;
    this._post({ type: "remove" });
    this._removed = true;
    this._worker?.terminate();
    this._resizeObserver.disconnect();
    this._handlers.remove();
    this._attribution?.remove();
    this._view.remove();
    this._container.classList.remove("mln-map");
    for (const resolve of this._queries.values()) resolve([]);
    this._queries.clear();
    this.fire("remove");
  }
}
