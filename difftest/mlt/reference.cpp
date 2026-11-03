// The C++ side of //difftest/mlt: decodes an MLT tile with
// mln::VectorMLTTileData (on the C++ MLT decoder, @maplibre_tile_spec//cpp)
// and with mlt::Decoder directly, and writes everything the port must
// reproduce as a line-based text dump. dump.rs writes the same dump from the
// Rust port. Byte strings are written as `x<hex>`, floats as their bits.

#include <mln/tile/geometry_tile_data.hpp>
#include <mln/tile/vector_mlt_tile_data.hpp>
#include <mln/util/logging.hpp>

#include <mlt/decoder.hpp>
#include <mlt/feature.hpp>
#include <mlt/geometry.hpp>
#include <mlt/layer.hpp>
#include <mlt/properties.hpp>
#include <mlt/tile.hpp>

#include <algorithm>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <exception>
#include <memory>
#include <string>
#include <utility>
#include <variant>
#include <vector>

using namespace mln;

std::string exceptionKind(std::exception_ptr e, std::string& what);

namespace {

thread_local std::string tileLog;

void hex(std::string& out, const std::string_view s) {
    static const char* digits = "0123456789abcdef";
    out += 'x';
    for (unsigned char c : s) {
        out += digits[c >> 4];
        out += digits[c & 15];
    }
}

class CaptureObserver : public Log::Observer {
public:
    bool onRecord(EventSeverity severity, Event event, int64_t, const std::string& msg) override {
        if (event == Event::ParseTile) {
            tileLog += "log " + std::to_string(static_cast<int>(severity)) + " ";
            hex(tileLog, msg);
            tileLog += "\n";
            return true;
        }
        return false;
    }
};

void installObserver() {
    static bool installed = [] {
        // Warnings go through the log thread by default; record them here.
        Log::useLogThread(false);
        Log::setObserver(std::make_unique<CaptureObserver>());
        return true;
    }();
    (void)installed;
}

void bits(std::string& out, double d) {
    uint64_t b;
    std::memcpy(&b, &d, sizeof b);
    char buf[24];
    std::snprintf(buf, sizeof buf, "%016llx", static_cast<unsigned long long>(b));
    out += buf;
}

void bitsf(std::string& out, float f) {
    uint32_t b;
    std::memcpy(&b, &f, sizeof b);
    char buf[16];
    std::snprintf(buf, sizeof buf, "%08x", b);
    out += buf;
}

void value(std::string& out, const Value& v) {
    v.match([&](const NullValue&) { out += "n"; },
            [&](bool b) { out += b ? "b1" : "b0"; },
            [&](uint64_t u) { out += "u" + std::to_string(u); },
            [&](int64_t i) { out += "i" + std::to_string(i); },
            [&](double d) {
                out += "d";
                bits(out, d);
            },
            [&](const std::string& s) {
                out += "s";
                hex(out, s);
            },
            [&](const auto&) { out += "?"; });
}

void identifier(std::string& out, const FeatureIdentifier& id) {
    id.match([&](const NullValue&) { out += "n"; },
             [&](uint64_t u) { out += "u" + std::to_string(u); },
             [&](int64_t i) { out += "i" + std::to_string(i); },
             [&](double d) {
                 out += "d";
                 bits(out, d);
             },
             [&](const std::string& s) {
                 out += "s";
                 hex(out, s);
             });
}

// "<prefix>! <kind> x<what>\n"
void exception(std::string& out, const std::string& prefix, std::exception_ptr e) {
    std::string what;
    out += prefix + "! " + exceptionKind(e, what) + " ";
    hex(out, what);
    out += "\n";
}

// mlt::Property, by alternative.
void property(std::string& out, const mlt::Property& p) {
    std::visit(
        [&](const auto& v) {
            using T = std::decay_t<decltype(v)>;
            if constexpr (std::is_same_v<T, std::nullptr_t>) {
                out += "null";
            } else if constexpr (std::is_same_v<T, bool>) {
                out += v ? "bool 1" : "bool 0";
            } else if constexpr (std::is_same_v<T, std::int32_t>) {
                out += "i32 " + std::to_string(v);
            } else if constexpr (std::is_same_v<T, std::uint32_t>) {
                out += "u32 " + std::to_string(v);
            } else if constexpr (std::is_same_v<T, std::int64_t>) {
                out += "i64 " + std::to_string(v);
            } else if constexpr (std::is_same_v<T, std::uint64_t>) {
                out += "u64 " + std::to_string(v);
            } else if constexpr (std::is_same_v<T, float>) {
                out += "f32 ";
                bitsf(out, v);
            } else if constexpr (std::is_same_v<T, double>) {
                out += "f64 ";
                bits(out, v);
            } else if constexpr (std::is_same_v<T, std::string_view>) {
                out += "str ";
                hex(out, v);
            } else {
                out += "optional";
            }
        },
        p);
}

void coords(std::string& out, const mlt::CoordVec& c) {
    out += std::to_string(c.size());
    for (const auto& p : c) {
        out += " ";
        bitsf(out, p.x);
        out += " ";
        bitsf(out, p.y);
    }
    out += "\n";
}

void rawGeometry(std::string& out, const mlt::geometry::Geometry& g) {
    using GT = mlt::metadata::tileset::GeometryType;
    out += "g " + std::to_string(static_cast<int>(g.type));
    out += " t " + std::to_string(g.getTriangles().size());
    for (auto i : g.getTriangles()) {
        out += " " + std::to_string(i);
    }
    out += "\n";
    switch (g.type) {
        case GT::POINT: {
            out += "c ";
            coords(out, {static_cast<const mlt::geometry::Point&>(g).getCoordinate()});
            break;
        }
        case GT::MULTIPOINT:
        case GT::LINESTRING:
            out += "c ";
            coords(out, static_cast<const mlt::geometry::MultiPoint&>(g).getCoordinates());
            break;
        case GT::POLYGON:
            for (const auto& r : static_cast<const mlt::geometry::Polygon&>(g).getRings()) {
                out += "c ";
                coords(out, r);
            }
            break;
        case GT::MULTILINESTRING:
            for (const auto& r : static_cast<const mlt::geometry::MultiLineString&>(g).getLineStrings()) {
                out += "c ";
                coords(out, r);
            }
            break;
        case GT::MULTIPOLYGON:
            for (const auto& p : static_cast<const mlt::geometry::MultiPolygon&>(g).getPolygons()) {
                out += "poly " + std::to_string(p.size()) + "\n";
                for (const auto& r : p) {
                    out += "c ";
                    coords(out, r);
                }
            }
            break;
        default:
            out += "unknown\n";
    }
}

// The mlt::MapLibreTile itself: what crate::mlt models.
void dumpRaw(std::string& out, const uint8_t* data, size_t size, bool fastPFOR) {
    std::unique_ptr<mlt::MapLibreTile> tile;
    try {
        tile = std::make_unique<mlt::MapLibreTile>(
            mlt::Decoder(fastPFOR).decode(mlt::DataView{reinterpret_cast<const char*>(data), size}));
    } catch (...) {
        exception(out, "raw", std::current_exception());
        return;
    }
    out += "rawlayers " + std::to_string(tile->getLayers().size()) + "\n";
    for (const auto& layer : tile->getLayers()) {
        out += "rl ";
        hex(out, layer.getName());
        out += " " + std::to_string(layer.getExtent()) + " " + std::to_string(layer.getFeatures().size()) + "\n";
        // Columns sorted by name (the map is unordered).
        std::vector<std::pair<std::string, const mlt::PresentProperties*>> columns;
        for (const auto& [key, column] : layer.getProperties()) {
            columns.emplace_back(key, &column);
        }
        std::sort(columns.begin(), columns.end(), [](const auto& a, const auto& b) { return a.first < b.first; });
        for (const auto& [key, column] : columns) {
            out += "col ";
            hex(out, key);
            out += " " + std::to_string(static_cast<int>(column->getType())) + "\n";
        }
        for (const auto& feature : layer.getFeatures()) {
            out += "rf ";
            const auto id = feature.getID();
            out += id ? "u" + std::to_string(*id) : std::string("n");
            out += " " + std::to_string(feature.getIndex()) + "\n";
            rawGeometry(out, feature.getGeometry());
            for (size_t c = 0; c < columns.size(); ++c) {
                try {
                    const auto p = columns[c].second->getProperty(feature.getIndex());
                    out += "rp " + std::to_string(c) + " ";
                    if (p) {
                        property(out, *p);
                    } else {
                        out += "none";
                    }
                    out += "\n";
                } catch (...) {
                    exception(out, "rp " + std::to_string(c), std::current_exception());
                }
            }
        }
    }
}

void rings(std::string& out, const GeometryCollection& g) {
    out += std::to_string(g.size()) + "\n";
    for (const auto& r : g) {
        out += "r " + std::to_string(r.size());
        for (const auto& p : r) {
            out += " " + std::to_string(p.x) + " " + std::to_string(p.y);
        }
        out += "\n";
    }
    out += "tri " + std::to_string(g.getTriangles().size());
    for (auto i : g.getTriangles()) {
        out += " " + std::to_string(i);
    }
    out += "\n";
}

const char* const extraKeys[] = {"name", "type", "class", "id", "zzz-missing", ""};

void dumpFeature(std::string& out, const GeometryTileLayer& layer, std::size_t i) {
    out += "f " + std::to_string(i) + "\n";
    std::unique_ptr<GeometryTileFeature> feature = layer.getFeature(i);
    out += "type " + std::to_string(static_cast<int>(feature->getType())) + "\n";
    out += "id ";
    identifier(out, feature->getID());
    out += "\n";

    std::vector<std::string> keys;
    try {
        const PropertyMap& props = feature->getProperties();
        std::vector<std::pair<std::string, Value>> sorted(props.begin(), props.end());
        std::sort(sorted.begin(), sorted.end(), [](const auto& a, const auto& b) { return a.first < b.first; });
        out += "props " + std::to_string(sorted.size()) + "\n";
        for (const auto& [k, v] : sorted) {
            out += "p ";
            hex(out, k);
            out += " ";
            value(out, v);
            out += "\n";
            keys.push_back(k);
        }
    } catch (...) {
        // A second call would return the columns read so far, in hash order
        // (see DEVIATIONS.md); not compared.
        exception(out, "props", std::current_exception());
    }
    for (const char* k : extraKeys) {
        keys.emplace_back(k);
    }
    for (const auto& k : keys) {
        try {
            auto v = feature->getValue(k);
            out += "v ";
            hex(out, k);
            out += " ";
            if (v) {
                value(out, *v);
            } else {
                out += "none";
            }
            out += "\n";
        } catch (...) {
            std::string prefix = "v ";
            hex(prefix, k);
            exception(out, prefix, std::current_exception());
        }
    }

    out += "geom ";
    rings(out, feature->getGeometries());
}

} // namespace

// Decodes `data` with VectorMLTTileData and mlt::Decoder and dumps both.
std::string mltDump(const uint8_t* data, size_t size, bool fastPFOR) {
    installObserver();
    std::string out;
    auto bytes = std::make_shared<const std::string>(reinterpret_cast<const char*>(data), size);
    VectorMLTTileData tile(bytes, fastPFOR);

    tileLog.clear();
    const auto names = tile.layerNames();
    out += tileLog;
    out += "names " + std::to_string(names.size()) + "\n";
    for (const auto& name : names) {
        out += "name ";
        hex(out, name);
        out += "\n";
    }

    for (const auto& name : names) {
        out += "layer ";
        hex(out, name);
        out += "\n";
        tileLog.clear();
        std::unique_ptr<GeometryTileLayer> layer = tile.getLayer(name);
        out += tileLog;
        if (!layer) {
            out += "nolayer\n";
            continue;
        }
        out += "info ";
        hex(out, layer->getName());
        out += " " + std::to_string(layer->featureCount()) + "\n";
        for (std::size_t i = 0; i < layer->featureCount(); ++i) {
            dumpFeature(out, *layer, i);
        }
    }

    tileLog.clear();
    auto missing = tile.getLayer("zzz-missing-layer");
    out += tileLog;
    out += missing ? "missing found\n" : "missing none\n";

    // A copy shares the decoded tile.
    std::unique_ptr<GeometryTileData> copy = tile.clone();
    out += "copy " + std::to_string(names.empty() ? 0 : (copy->getLayer(names.front()) ? 1 : 0)) + "\n";

    dumpRaw(out, data, size, fastPFOR);
    return out;
}

bool mltNdebug() {
#ifdef NDEBUG
    return true;
#else
    return false;
#endif
}
