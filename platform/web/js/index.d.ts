// Types of @bartlwrs/mln-next-web: a trimmed-down MapLibre GL JS API.

export type LngLatLike = LngLat | [number, number] | { lng: number; lat: number } | { lon: number; lat: number };
export type LngLatBoundsLike = LngLatBounds | [LngLatLike, LngLatLike] | [number, number, number, number];
export type PointLike = Point | [number, number];
export type PaddingOptions = { top: number; bottom: number; left: number; right: number };

export class Point {
  constructor(x: number, y: number);
  x: number;
  y: number;
  static convert(value: PointLike | { x: number; y: number }): Point;
  add(p: Point): Point;
  sub(p: Point): Point;
  mult(k: number): Point;
  dist(p: Point): number;
  equals(p: Point): boolean;
}

export class LngLat {
  constructor(lng: number, lat: number);
  lng: number;
  lat: number;
  static convert(value: LngLatLike): LngLat;
  wrap(): LngLat;
  toArray(): [number, number];
}

export class LngLatBounds {
  constructor(sw?: LngLatLike | [number, number, number, number], ne?: LngLatLike);
  static convert(value: LngLatBoundsLike): LngLatBounds;
  extend(value: LngLatLike | LngLatBounds): this;
  getSouthWest(): LngLat;
  getNorthEast(): LngLat;
  getNorthWest(): LngLat;
  getSouthEast(): LngLat;
  getWest(): number;
  getSouth(): number;
  getEast(): number;
  getNorth(): number;
  getCenter(): LngLat;
  contains(value: LngLatLike): boolean;
  toArray(): [[number, number], [number, number]];
}

export interface MapEvent {
  type: string;
  target: Map;
  [key: string]: unknown;
}

export interface MapMouseEvent extends MapEvent {
  point: Point;
  lngLat: LngLat;
  originalEvent: MouseEvent;
}

export class Evented {
  on(type: string, listener: (event: MapEvent) => void): this;
  once(type: string, listener: (event: MapEvent) => void): this;
  once(type: string): Promise<MapEvent>;
  off(type: string, listener: (event: MapEvent) => void): this;
  listens(type: string): boolean;
  fire(type: string, properties?: object): this;
}

export interface Handler {
  enable(): void;
  disable(): void;
  isEnabled(): boolean;
}

export interface MapOptions {
  container: HTMLElement | string;
  style?: string | object;
  center?: LngLatLike;
  zoom?: number;
  bearing?: number;
  pitch?: number;
  minZoom?: number;
  maxZoom?: number;
  minPitch?: number;
  maxPitch?: number;
  interactive?: boolean;
  attributionControl?: false | { customAttribution?: string | string[] };
  pixelRatio?: number;
  /** Run in workers; the default is to when the page is cross-origin isolated. */
  threads?: boolean;
}

export interface CameraOptions {
  center?: LngLatLike;
  zoom?: number;
  bearing?: number;
  pitch?: number;
}

export interface AnimationOptions {
  duration?: number;
  animate?: boolean;
}

export interface FlyToOptions extends CameraOptions, AnimationOptions {
  speed?: number;
  curve?: number;
}

export interface FitBoundsOptions extends AnimationOptions {
  padding?: number | Partial<PaddingOptions>;
  maxZoom?: number;
  bearing?: number;
  pitch?: number;
}

export interface QueriedFeature {
  type: "Feature";
  id?: string | number;
  geometry: object;
  properties: Record<string, unknown>;
  source: string;
  sourceLayer: string;
}

export class Map extends Evented {
  constructor(options: MapOptions);
  dragPan: Handler;
  scrollZoom: Handler;
  doubleClickZoom: Handler;
  dragRotate: Handler;
  touchZoomRotate: Handler;
  keyboard: Handler;

  getContainer(): HTMLElement;
  getCanvasContainer(): HTMLElement;
  getCanvas(): HTMLCanvasElement;
  getCenter(): LngLat;
  getZoom(): number;
  getBearing(): number;
  getPitch(): number;
  getBounds(): LngLatBounds;
  getMinZoom(): number;
  getMaxZoom(): number;
  getMinPitch(): number;
  getMaxPitch(): number;
  loaded(): boolean;
  isMoving(): boolean;
  project(lngLat: LngLatLike): Point;
  unproject(point: PointLike): LngLat;

  jumpTo(options: CameraOptions): this;
  easeTo(options: CameraOptions & AnimationOptions): this;
  flyTo(options: FlyToOptions): this;
  panBy(offset: PointLike, options?: AnimationOptions): this;
  panTo(lngLat: LngLatLike, options?: AnimationOptions): this;
  zoomTo(zoom: number, options?: AnimationOptions): this;
  zoomIn(options?: AnimationOptions): this;
  zoomOut(options?: AnimationOptions): this;
  rotateTo(bearing: number, options?: AnimationOptions): this;
  resetNorth(options?: AnimationOptions): this;
  resetNorthPitch(options?: AnimationOptions): this;
  setCenter(center: LngLatLike): this;
  setZoom(zoom: number): this;
  setBearing(bearing: number): this;
  setPitch(pitch: number): this;
  fitBounds(bounds: LngLatBoundsLike, options?: FitBoundsOptions): this;
  stop(): this;
  setMinZoom(zoom?: number | null): this;
  setMaxZoom(zoom?: number | null): this;
  setMinPitch(pitch?: number | null): this;
  setMaxPitch(pitch?: number | null): this;

  setStyle(style: string | object): this;
  resize(): this;
  triggerRepaint(): void;
  queryRenderedFeatures(geometry?: PointLike | [PointLike, PointLike], options?: { layers?: string[] }): Promise<QueriedFeature[]>;
  queryRenderedFeatures(options: { layers?: string[] }): Promise<QueriedFeature[]>;
  remove(): void;
}
