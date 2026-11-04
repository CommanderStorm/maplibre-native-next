// Scroll zoom, as MapLibre GL JS's ScrollZoomHandler does it, split in two:
//
// - The page (handlers.js, WheelClassifier) tells a mouse wheel from a
//   trackpad by the size and timing of the wheel events, and passes their
//   deltas on.
// - The engine's loop (engine.js, ScrollZoom) accumulates the deltas and, each
//   animation frame right before the map draws, turns them into a zoom: a
//   sigmoid of the delta (at most a factor of 2 per frame), eased over 200 ms
//   from the last wheel event for a mouse wheel, applied at once for a
//   trackpad. Zooming keeps the point under the pointer in place.

// A mouse wheel step in Firefox on macOS, and some other wheels, are
// multiples of this.
export const WHEEL_ZOOM_DELTA = 4.000244140625;
// Zoom levels per delta pixel, through the sigmoid: trackpads and single
// wheel steps, and runs of wheel steps.
const DEFAULT_ZOOM_RATE = 1 / 100;
const WHEEL_ZOOM_RATE = 1 / 450;
const MAX_SCALE_PER_FRAME = 2;
const WHEEL_EASE_MS = 200;

// A cubic Bézier timing function (WebKit's UnitBezier, as GL JS uses).
function bezier(p1x, p1y, p2x, p2y) {
  const cx = 3 * p1x;
  const bx = 3 * (p2x - p1x) - cx;
  const ax = 1 - cx - bx;
  const cy = 3 * p1y;
  const by = 3 * (p2y - p1y) - cy;
  const ay = 1 - cy - by;
  const sampleX = (t) => ((ax * t + bx) * t + cx) * t;
  const sampleY = (t) => ((ay * t + by) * t + cy) * t;
  const derivativeX = (t) => (3 * ax * t + 2 * bx) * t + cx;
  const solveX = (x) => {
    if (x <= 0) return 0;
    if (x >= 1) return 1;
    let t = x;
    for (let i = 0; i < 8; i++) {
      const error = sampleX(t) - x;
      if (Math.abs(error) < 1e-6) return t;
      const d = derivativeX(t);
      if (Math.abs(d) < 1e-6) break;
      t -= error / d;
    }
    let low = 0;
    let high = 1;
    t = x;
    for (let i = 0; i < 20; i++) {
      const value = sampleX(t);
      if (Math.abs(value - x) < 1e-6) break;
      if (x > value) low = t;
      else high = t;
      t = (high - low) / 2 + low;
    }
    return t;
  };
  return (x) => sampleY(solveX(x));
}

const DEFAULT_EASING = bezier(0.25, 0.1, 0.25, 1);

/** The page's half: classifies wheel events and passes their deltas on. */
export class WheelClassifier {
  /** `send({value, kind, x, y})` takes a delta to apply. */
  constructor(send) {
    this._send = send;
    this._type = null;
    this._lastTime = 0;
    this._lastValue = 0;
    this._timeout = null;
  }

  /** A wheel event at the CSS pixel `point`. */
  wheel(event, point) {
    let value = event.deltaMode === WheelEvent.DOM_DELTA_LINE ? event.deltaY * 40 : event.deltaY;
    const now = performance.now();
    const timeDelta = now - this._lastTime;
    this._lastTime = now;
    if (value !== 0 && value % WHEEL_ZOOM_DELTA === 0) {
      // Definitely a mouse wheel.
      this._type = "wheel";
    } else if (value !== 0 && Math.abs(value) < 4) {
      // So small that it is a trackpad.
      this._type = "trackpad";
    } else if (timeDelta > 400) {
      // Likely a new scroll. Wait up to 40 ms for the next event to tell; a
      // lone event is a wheel step.
      this._type = null;
      this._lastValue = value;
      this._timeout = setTimeout(() => {
        this._timeout = null;
        this._type = "wheel";
        this._send({ value: this._lastValue, kind: "wheel", x: point.x, y: point.y });
      }, 40);
    } else if (!this._type) {
      // A repeat of unknown type: little change per time is a fast trackpad.
      this._type = Math.abs(timeDelta * value) < 200 ? "trackpad" : "wheel";
      // The delayed first event is part of this one.
      if (this._timeout) {
        clearTimeout(this._timeout);
        this._timeout = null;
        value += this._lastValue;
      }
    }
    // Slower with shift, for precise zooming.
    if (event.shiftKey && value) value /= 4;
    if (this._type) this._send({ value, kind: this._type, x: point.x, y: point.y });
  }
}

/** The engine loop's half: turns the deltas into zoom steps, one a frame. */
export class ScrollZoom {
  constructor(engine) {
    this._engine = engine;
    this._delta = 0;
    this._type = null;
    this._active = false;
    this._around = null;
    this._targetZoom = null;
    this._startZoom = null;
    this._easing = null;
    this._prevEase = null;
    this._lastWheelTime = 0;
    this._finishTimeout = null;
    this.minZoom = 0;
    this.maxZoom = 22;
  }

  /** A delta from the page: `value` pixels of `kind` ("wheel" or "trackpad"). */
  wheel({ value, kind, x, y }) {
    this._type = kind;
    this._lastWheelTime = performance.now();
    this._delta -= value;
    if (this._active || this._delta === 0) return;
    this._active = true;
    this._around = { x, y };
    if (this._finishTimeout) {
      clearTimeout(this._finishTimeout);
      this._finishTimeout = null;
    } else {
      this._engine.cancel_transitions();
      this._engine.set_gesture_in_progress(true);
    }
  }

  /** Applies this frame's zoom step; call right before the frame draws. */
  frame() {
    if (!this._active) return;
    const zoom = this._engine.get_zoom();
    if (this._delta !== 0) {
      // Trackpads and single wheel steps zoom faster per pixel.
      const rate = this._type === "wheel" && Math.abs(this._delta) > WHEEL_ZOOM_DELTA ? WHEEL_ZOOM_RATE : DEFAULT_ZOOM_RATE;
      let scale = MAX_SCALE_PER_FRAME / (1 + Math.exp(-Math.abs(this._delta * rate)));
      if (this._delta < 0 && scale !== 0) scale = 1 / scale;
      const from = this._targetZoom ?? zoom;
      this._targetZoom = Math.min(this.maxZoom, Math.max(this.minZoom, from + Math.log2(scale)));
      // A wheel eases from where the map is now to the new target.
      if (this._type === "wheel") {
        this._startZoom = zoom;
        this._easing = this._smoothOutEasing(WHEEL_EASE_MS);
      }
      this._delta = 0;
    }
    const target = this._targetZoom ?? zoom;
    let next = target;
    let finished = true;
    if (this._type === "wheel" && this._startZoom !== null && this._easing) {
      const t = Math.min((performance.now() - this._lastWheelTime) / WHEEL_EASE_MS, 1);
      next = this._startZoom + (target - this._startZoom) * this._easing(t);
      finished = t >= 1;
    }
    this._engine.zoom_around(next, this._around.x, this._around.y);
    if (finished) {
      this._active = false;
      // A wheel event within 200 ms continues from the same target.
      this._finishTimeout = setTimeout(() => {
        this._finishTimeout = null;
        this._targetZoom = null;
        this._startZoom = null;
        this._easing = null;
        this._engine.set_gesture_in_progress(false);
      }, 200);
    }
  }

  // An ease-out that starts at the current ease's speed, so that wheel steps
  // in a row zoom without a jolt (GL JS's _smoothOutEasing).
  _smoothOutEasing(duration) {
    let easing = DEFAULT_EASING;
    if (this._prevEase) {
      const previous = this._prevEase;
      const t = (performance.now() - previous.start) / previous.duration;
      const speed = previous.easing(t + 0.01) - previous.easing(t);
      const x = (0.27 / Math.sqrt(speed * speed + 0.0001)) * 0.01;
      const y = Math.sqrt(0.27 * 0.27 - x * x);
      easing = bezier(x, y, 0.25, 1);
    }
    this._prevEase = { start: performance.now(), duration, easing };
    return easing;
  }
}
