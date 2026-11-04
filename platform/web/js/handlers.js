// Mouse, touch and keyboard interaction, as MapLibre GL JS's default
// handlers: drag to pan (with inertia), scroll to zoom, double-click to zoom
// in, right- or control-drag to rotate and pitch, pinch to zoom and rotate,
// and the arrow and +/- keys. Each is a property of the Map with enable(),
// disable() and isEnabled().
//
// The handlers send gestures to the engine (move_by, scale_by, rotate_by,
// pitch_by), which applies them to the camera as the core's gestures do; the
// camera then comes back with the next frame.

import { Point } from "./geo.js";
import { WheelClassifier } from "./scroll_zoom.js";

class Toggle {
  constructor(enabled) {
    this._enabled = enabled;
  }

  enable() {
    this._enabled = true;
  }

  disable() {
    this._enabled = false;
  }

  isEnabled() {
    return this._enabled;
  }
}

// GL JS's inertia settings: a pan keeps going after release at this
// deceleration (px/s²), for velocities up to maxSpeed (px/s).
const INERTIA = { linearity: 0.3, deceleration: 2500, maxSpeed: 1400 };
const DEGREES_PER_PIXEL_ROTATE = 0.8;
const DEGREES_PER_PIXEL_PITCH = 0.5;
// A press that moves less than this (CSS px) is a click.
const CLICK_TOLERANCE = 3;

export class Handlers {
  constructor(map, element, interactive) {
    this._map = map;
    this._element = element;
    this.dragPan = new Toggle(interactive);
    this.scrollZoom = new Toggle(interactive);
    this.doubleClickZoom = new Toggle(interactive);
    this.dragRotate = new Toggle(interactive);
    this.touchZoomRotate = new Toggle(interactive);
    this.keyboard = new Toggle(interactive);
    this._pointers = new globalThis.Map();
    this._mode = null;
    this._samples = [];
    this._listeners = [];
    this._wheelClassifier = new WheelClassifier((delta) => map._post({ type: "wheel", ...delta }));

    this._listen("pointerdown", (e) => this._pointerDown(e));
    this._listen("pointermove", (e) => this._pointerMove(e));
    this._listen("pointerup", (e) => this._pointerUp(e));
    this._listen("pointercancel", (e) => this._pointerUp(e));
    this._listen("wheel", (e) => this._wheel(e), { passive: false });
    this._listen("dblclick", (e) => this._dblclick(e));
    this._listen("contextmenu", (e) => {
      e.preventDefault();
      map._fireMouse("contextmenu", e);
    });
    this._listen("keydown", (e) => this._keydown(e));
  }

  remove() {
    for (const [type, listener, options] of this._listeners) this._element.removeEventListener(type, listener, options);
    this._listeners = [];
  }

  _listen(type, listener, options) {
    this._element.addEventListener(type, listener, options);
    this._listeners.push([type, listener, options]);
  }

  _point(e) {
    const rect = this._element.getBoundingClientRect();
    return new Point(e.clientX - rect.left, e.clientY - rect.top);
  }

  _pointerDown(e) {
    this._map._fireMouse("mousedown", e);
    this._element.focus({ preventScroll: true });
    this._element.setPointerCapture(e.pointerId);
    this._pointers.set(e.pointerId, this._point(e));
    this._map._call("cancel_transitions");
    if (this._pointers.size === 1) {
      this._press = { point: this._point(e), time: performance.now(), moved: false };
      const rotate = e.pointerType === "mouse" && (e.button === 2 || (e.button === 0 && e.ctrlKey));
      if (rotate && this.dragRotate.isEnabled()) this._begin("rotate");
      else if (e.button === 0 && this.dragPan.isEnabled()) this._begin("pan");
    } else if (this._pointers.size === 2 && this.touchZoomRotate.isEnabled()) {
      this._begin("pinch");
      this._pinch = this._pinchState();
    }
  }

  _begin(mode) {
    if (!this._mode) this._map._call("set_gesture_in_progress", true);
    this._mode = mode;
    this._samples = [];
    this._element.classList.add("mln-grabbing");
  }

  _end() {
    if (!this._mode) return;
    this._mode = null;
    this._element.classList.remove("mln-grabbing");
    this._map._call("set_gesture_in_progress", false);
  }

  _pointerMove(e) {
    const point = this._point(e);
    if (!this._pointers.has(e.pointerId)) {
      this._map._fireMouse("mousemove", e);
      return;
    }
    const last = this._pointers.get(e.pointerId);
    this._pointers.set(e.pointerId, point);
    if (this._press && point.dist(this._press.point) > CLICK_TOLERANCE) this._press.moved = true;
    this._map._fireMouse("mousemove", e);
    const dx = point.x - last.x;
    const dy = point.y - last.y;
    if (this._mode === "pan" && this._pointers.size === 1) {
      if (dx === 0 && dy === 0) return;
      this._map._call("move_by", dx, dy, NaN);
      this._samples.push({ time: performance.now(), dx, dy });
    } else if (this._mode === "rotate") {
      if (dx !== 0) this._rotate(dx * DEGREES_PER_PIXEL_ROTATE);
      if (dy !== 0) this._map._call("pitch_by", -dy * DEGREES_PER_PIXEL_PITCH, NaN);
    } else if (this._mode === "pinch" && this._pointers.size === 2) {
      const now = this._pinchState();
      const before = this._pinch;
      this._pinch = now;
      this._map._call("move_by", now.center.x - before.center.x, now.center.y - before.center.y, NaN);
      if (before.distance > 0) this._map._call("scale_by", now.distance / before.distance, now.center.x, now.center.y, NaN);
      this._rotate(((now.angle - before.angle) * 180) / Math.PI);
    }
  }

  _pointerUp(e) {
    if (!this._pointers.has(e.pointerId)) return;
    this._pointers.delete(e.pointerId);
    this._map._fireMouse("mouseup", e);
    const press = this._press;
    if (this._pointers.size === 0) {
      this._press = null;
      if (this._mode === "pan") this._inertia();
      this._end();
      if (press && !press.moved && e.button === 0) this._map._fireMouse("click", e);
    } else if (this._mode === "pinch" && this._pointers.size === 1 && this.dragPan.isEnabled()) {
      this._mode = "pan";
      this._samples = [];
    }
  }

  // Keeps a pan going after release, slowing down, as GL JS does.
  _inertia() {
    const now = performance.now();
    const recent = this._samples.filter((s) => now - s.time < 160);
    if (recent.length < 2) return;
    const seconds = (now - recent[0].time) / 1000;
    if (seconds <= 0) return;
    let vx = recent.reduce((sum, s) => sum + s.dx, 0) / seconds;
    let vy = recent.reduce((sum, s) => sum + s.dy, 0) / seconds;
    const speed = Math.hypot(vx, vy);
    if (speed < 50) return;
    if (speed > INERTIA.maxSpeed) {
      vx *= INERTIA.maxSpeed / speed;
      vy *= INERTIA.maxSpeed / speed;
    }
    const duration = Math.min(speed, INERTIA.maxSpeed) / (INERTIA.deceleration * INERTIA.linearity);
    this._map._call("move_by", (vx * duration) / 2, (vy * duration) / 2, duration * 1000);
  }

  _pinchState() {
    const [a, b] = [...this._pointers.values()];
    return {
      center: new Point((a.x + b.x) / 2, (a.y + b.y) / 2),
      distance: a.dist(b),
      angle: Math.atan2(b.y - a.y, b.x - a.x),
    };
  }

  // Rotates the map by `degrees` clockwise on screen, around its center.
  _rotate(degrees) {
    if (degrees === 0) return;
    const { width, height } = this._map._size();
    const cx = width / 2;
    const cy = height / 2;
    const r = Math.max(50, Math.min(width, height) / 4);
    const radians = (degrees * Math.PI) / 180;
    this._map._call("rotate_by", cx + r, cy, cx + r * Math.cos(radians), cy + r * Math.sin(radians));
  }

  // GL JS's scroll zoom (scroll_zoom.js): the engine eases toward the zoom
  // the deltas add up to, around the pointer.
  _wheel(e) {
    if (!this.scrollZoom.isEnabled()) return;
    e.preventDefault();
    this._wheelClassifier.wheel(e, this._point(e));
  }

  _dblclick(e) {
    this._map._fireMouse("dblclick", e);
    if (!this.doubleClickZoom.isEnabled()) return;
    e.preventDefault();
    const point = this._point(e);
    this._map._call("scale_by", e.shiftKey ? 0.5 : 2, point.x, point.y, 300);
  }

  _keydown(e) {
    if (!this.keyboard.isEnabled() || e.altKey || e.ctrlKey || e.metaKey) return;
    const pan = 100;
    switch (e.key) {
      case "+":
      case "=":
        this._map._call("scale_by", 2, NaN, NaN, 300);
        break;
      case "-":
      case "_":
        this._map._call("scale_by", 0.5, NaN, NaN, 300);
        break;
      case "ArrowLeft":
        if (e.shiftKey) this._rotate(-15);
        else this._map._call("move_by", pan, 0, 300);
        break;
      case "ArrowRight":
        if (e.shiftKey) this._rotate(15);
        else this._map._call("move_by", -pan, 0, 300);
        break;
      case "ArrowUp":
        if (e.shiftKey) this._map._call("pitch_by", 10, 300);
        else this._map._call("move_by", 0, pan, 300);
        break;
      case "ArrowDown":
        if (e.shiftKey) this._map._call("pitch_by", -10, 300);
        else this._map._call("move_by", 0, -pan, 300);
        break;
      default:
        return;
    }
    e.preventDefault();
  }
}
