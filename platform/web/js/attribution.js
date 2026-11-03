// The attribution in the map's bottom-right corner: the style's sources'
// attributions (HTML, as GL JS shows them) and any custom attribution.
export class AttributionControl {
  constructor(map, options = {}) {
    this._custom = [].concat(options.customAttribution ?? []);
    this._sources = [];
    this._element = document.createElement("div");
    this._element.className = "mln-attribution";
    map.getContainer().appendChild(this._element);
    this._render();
  }

  setSourceAttributions(attributions) {
    this._sources = attributions;
    this._render();
  }

  remove() {
    this._element.remove();
  }

  _render() {
    const parts = [...this._custom, ...this._sources].filter((part, i, all) => part && all.indexOf(part) === i);
    this._element.innerHTML = parts.join(" | ");
    this._element.hidden = parts.length === 0;
  }
}
