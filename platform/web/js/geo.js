// LngLat, LngLatBounds and Point as in MapLibre GL JS, and the projection
// between them and the map's pixels.
//
// The engine reports the transform's coordinate matrix and its inverse with
// every camera change (Engine.state), so project and unproject here compute
// what the core's latLngToScreenCoordinate and screenCoordinateToLatLng do,
// synchronously, for the last frame's camera.

const MAX_LATITUDE = 85.051128779806604;

export class Point {
  constructor(x, y) {
    this.x = x;
    this.y = y;
  }

  static convert(value) {
    if (value instanceof Point) return value;
    if (Array.isArray(value)) return new Point(value[0], value[1]);
    return new Point(value.x, value.y);
  }

  add(p) {
    return new Point(this.x + p.x, this.y + p.y);
  }

  sub(p) {
    return new Point(this.x - p.x, this.y - p.y);
  }

  mult(k) {
    return new Point(this.x * k, this.y * k);
  }

  dist(p) {
    return Math.hypot(this.x - p.x, this.y - p.y);
  }

  equals(p) {
    return this.x === p.x && this.y === p.y;
  }
}

export class LngLat {
  constructor(lng, lat) {
    if (!Number.isFinite(lng) || !Number.isFinite(lat)) throw new Error(`Invalid LngLat: (${lng}, ${lat})`);
    if (lat > 90 || lat < -90) throw new Error("Invalid LngLat latitude: must be between -90 and 90");
    this.lng = lng;
    this.lat = lat;
  }

  /** A LngLat from a LngLat, [lng, lat], {lng, lat} or {lon, lat}. */
  static convert(value) {
    if (value instanceof LngLat) return value;
    if (Array.isArray(value) && value.length >= 2) return new LngLat(Number(value[0]), Number(value[1]));
    if (value && typeof value === "object" && "lat" in value) {
      return new LngLat(Number("lng" in value ? value.lng : value.lon), Number(value.lat));
    }
    throw new Error("`LngLatLike` argument must be a LngLat, an object {lng, lat} or {lon, lat}, or [lng, lat]");
  }

  /** The same place with the longitude in [-180, 180). */
  wrap() {
    return new LngLat(((((this.lng + 180) % 360) + 360) % 360) - 180, this.lat);
  }

  toArray() {
    return [this.lng, this.lat];
  }

  toString() {
    return `LngLat(${this.lng}, ${this.lat})`;
  }
}

export class LngLatBounds {
  /** From a south-west and a north-east LngLatLike, or [w, s, e, n]. */
  constructor(sw, ne) {
    if (Array.isArray(sw) && sw.length === 4 && ne === undefined) {
      ne = [sw[2], sw[3]];
      sw = [sw[0], sw[1]];
    }
    if (sw !== undefined) this.extend(sw);
    if (ne !== undefined) this.extend(ne);
  }

  static convert(value) {
    if (value instanceof LngLatBounds) return value;
    if (Array.isArray(value) && value.length === 2) return new LngLatBounds(value[0], value[1]);
    return new LngLatBounds(value);
  }

  extend(value) {
    const points = value instanceof LngLatBounds ? [value._sw, value._ne] : [LngLat.convert(value)];
    for (const p of points) {
      if (!p) continue;
      this._sw = this._sw ? new LngLat(Math.min(this._sw.lng, p.lng), Math.min(this._sw.lat, p.lat)) : new LngLat(p.lng, p.lat);
      this._ne = this._ne ? new LngLat(Math.max(this._ne.lng, p.lng), Math.max(this._ne.lat, p.lat)) : new LngLat(p.lng, p.lat);
    }
    return this;
  }

  getSouthWest() { return this._sw; }
  getNorthEast() { return this._ne; }
  getNorthWest() { return new LngLat(this._sw.lng, this._ne.lat); }
  getSouthEast() { return new LngLat(this._ne.lng, this._sw.lat); }
  getWest() { return this._sw.lng; }
  getSouth() { return this._sw.lat; }
  getEast() { return this._ne.lng; }
  getNorth() { return this._ne.lat; }

  getCenter() {
    return new LngLat((this._sw.lng + this._ne.lng) / 2, (this._sw.lat + this._ne.lat) / 2);
  }

  contains(value) {
    const { lng, lat } = LngLat.convert(value);
    return lat >= this._sw.lat && lat <= this._ne.lat && lng >= this._sw.lng && lng <= this._ne.lng;
  }

  toArray() {
    return [this._sw.toArray(), this._ne.toArray()];
  }
}

// transform_mat4 (gl-matrix): `m` column-major.
function transform(m, x, y, z, w) {
  return [
    m[0] * x + m[4] * y + m[8] * z + m[12] * w,
    m[1] * x + m[5] * y + m[9] * z + m[13] * w,
    m[2] * x + m[6] * y + m[10] * z + m[14] * w,
    m[3] * x + m[7] * y + m[11] * z + m[15] * w,
  ];
}

/**
 * A camera and projection the engine reported: center, zoom, bearing and
 * pitch, the size in CSS pixels, the scale (2^zoom) and the transform's
 * coordinate matrix and its inverse.
 */
export class Projection {
  constructor(state) {
    this.center = new LngLat(state[0], state[1]);
    this.zoom = state[2];
    // The core keeps bearings in [0, 360); GL JS reports (-180, 180].
    this.bearing = state[3] > 180 ? state[3] - 360 : state[3];
    this.pitch = state[4];
    this.width = state[5];
    this.height = state[6];
    this.scale = state[7];
    this.matrix = state.slice(8, 24);
    this.inverse = state.slice(24, 40);
  }

  /** The CSS pixel of a LngLat (TransformState::latLngToScreenCoordinate). */
  project(lngLat) {
    const lat = Math.max(-MAX_LATITUDE, Math.min(MAX_LATITUDE, lngLat.lat));
    const x = ((lngLat.lng + 180) / 360) * this.scale;
    const y = ((180 - (180 / Math.PI) * Math.log(Math.tan(Math.PI / 4 + (lat * Math.PI) / 360))) / 360) * this.scale;
    const p = transform(this.matrix, x, y, 0, 1);
    return new Point(p[0] / p[3], this.height - p[1] / p[3]);
  }

  /** The LngLat at a CSS pixel (TransformState::screenCoordinateToLatLng). */
  unproject(point) {
    const flippedY = this.height - point.y;
    const c0 = transform(this.inverse, point.x, flippedY, 0, 1);
    const c1 = transform(this.inverse, point.x, flippedY, 1, 1);
    const [x0, y0, z0] = [c0[0] / c0[3], c0[1] / c0[3], c0[2] / c0[3]];
    const [x1, y1, z1] = [c1[0] / c1[3], c1[1] / c1[3], c1[2] / c1[3]];
    const t = z0 <= z1 ? 0 : (0 - z0) / (z1 - z0);
    // A tile coordinate at zoom 0: the world is [0, 1).
    const x = (x0 + (x1 - x0) * t) / this.scale;
    const y = (y0 + (y1 - y0) * t) / this.scale;
    const lng = x * 360 - 180;
    const lat = (360 / Math.PI) * Math.atan(Math.exp(((180 - y * 360) * Math.PI) / 180)) - 90;
    return new LngLat(lng, Math.max(-90, Math.min(90, lat)));
  }
}

