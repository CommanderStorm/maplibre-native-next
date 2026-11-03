// on/once/off and fire, as MapLibre GL JS's Evented has them.
export class Evented {
  constructor() {
    this._listeners = new globalThis.Map();
  }

  /** Calls `listener` with each event of `type`. */
  on(type, listener) {
    if (!this._listeners.has(type)) this._listeners.set(type, new Set());
    this._listeners.get(type).add(listener);
    return this;
  }

  /** Calls `listener` with the next event of `type`, or returns a promise of it. */
  once(type, listener) {
    if (!listener) return new Promise((resolve) => this.once(type, resolve));
    const wrapped = (event) => {
      this.off(type, wrapped);
      listener.call(this, event);
    };
    return this.on(type, wrapped);
  }

  off(type, listener) {
    this._listeners.get(type)?.delete(listener);
    return this;
  }

  listens(type) {
    return (this._listeners.get(type)?.size ?? 0) > 0;
  }

  fire(type, properties = {}) {
    const listeners = this._listeners.get(type);
    if (!listeners || listeners.size === 0) {
      // As GL JS does, unhandled errors go to the console.
      if (type === "error") console.error(properties.error ?? properties);
      return this;
    }
    const event = { type, target: this, ...properties };
    for (const listener of [...listeners]) listener.call(this, event);
    return this;
  }
}
