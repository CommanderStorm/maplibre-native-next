// The Rust half of reference.cpp's dump: the same text, written from
// //crates/mln_tile's VectorMLTTileData and its mlt::MapLibreTile model.

use maplibre_native_base::{Identifier, Value};
use mln_tile::VectorMLTTileData;
use mln_tile::mlt::{self, CoordVec, MapLibreTile, Property, Shape};
use mln_tile_id::{GeometryCollection, GeometryTileData, GeometryTileFeature, GeometryTileLayer};
use mln_util::logging::{Event, EventSeverity, Log, Observer};
use std::cell::RefCell;
use std::fmt::Write;
use std::sync::Arc;

pub fn hex(out: &mut String, bytes: &[u8]) {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    out.push('x');
    for &b in bytes {
        out.push(DIGITS[(b >> 4) as usize] as char);
        out.push(DIGITS[(b & 15) as usize] as char);
    }
}

pub fn unhex(s: &str) -> Vec<u8> {
    let s = s.strip_prefix('x').unwrap_or(s);
    (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap_or(b'?')).collect()
}

fn value(out: &mut String, v: &Value) {
    match v {
        Value::Null => out.push('n'),
        Value::Bool(b) => out.push_str(if *b { "b1" } else { "b0" }),
        Value::Uint(u) => {
            let _ = write!(out, "u{u}");
        }
        Value::Int(i) => {
            let _ = write!(out, "i{i}");
        }
        Value::Double(d) => {
            let _ = write!(out, "d{:016x}", d.to_bits());
        }
        Value::String(s) => {
            out.push('s');
            hex(out, s.as_bytes());
        }
        _ => out.push('?'),
    }
}

fn identifier(out: &mut String, id: &Identifier) {
    match id {
        Identifier::Null => out.push('n'),
        Identifier::Uint(u) => {
            let _ = write!(out, "u{u}");
        }
        Identifier::Int(i) => {
            let _ = write!(out, "i{i}");
        }
        Identifier::Double(d) => {
            let _ = write!(out, "d{:016x}", d.to_bits());
        }
        Identifier::String(s) => {
            out.push('s');
            hex(out, s.as_bytes());
        }
    }
}

/// `<prefix>! runtime x<what>`: every exception of the port is a
/// `std::runtime_error` in C++.
fn exception(out: &mut String, prefix: &str, e: &mlt::Error) {
    out.push_str(prefix);
    out.push_str("! runtime ");
    hex(out, e.what().as_bytes());
    out.push('\n');
}

thread_local! {
    static TILE_LOG: RefCell<String> = const { RefCell::new(String::new()) };
}

struct Capture;

impl Observer for Capture {
    fn on_record(&self, severity: EventSeverity, event: Event, _code: i64, msg: &str) -> bool {
        if event == Event::ParseTile {
            TILE_LOG.with(|l| {
                let mut l = l.borrow_mut();
                let _ = write!(l, "log {} ", severity as u8);
                hex(&mut l, msg.as_bytes());
                l.push('\n');
            });
            return true;
        }
        false
    }
}

/// Runs `f` and appends the ParseTile log records it makes.
fn logged<T>(out: &mut String, f: impl FnOnce() -> T) -> T {
    TILE_LOG.with(|l| l.borrow_mut().clear());
    let previous = Log::set_thread_observer(Some(Box::new(Capture)));
    let r = f();
    Log::set_thread_observer(previous);
    TILE_LOG.with(|l| out.push_str(&l.borrow()));
    r
}

fn property(out: &mut String, p: &Property<'_>) {
    let _ = match p {
        Property::Bool(b) => write!(out, "bool {}", u8::from(*b)),
        Property::I32(v) => write!(out, "i32 {v}"),
        Property::U32(v) => write!(out, "u32 {v}"),
        Property::I64(v) => write!(out, "i64 {v}"),
        Property::U64(v) => write!(out, "u64 {v}"),
        Property::F32(v) => write!(out, "f32 {:08x}", v.to_bits()),
        Property::F64(v) => write!(out, "f64 {:016x}", v.to_bits()),
        Property::Str(s) => {
            out.push_str("str ");
            hex(out, s.as_bytes());
            Ok(())
        }
    };
}

fn coords(out: &mut String, c: &CoordVec) {
    let _ = write!(out, "c {}", c.len());
    for p in c {
        let _ = write!(out, " {:08x} {:08x}", p.x.to_bits(), p.y.to_bits());
    }
    out.push('\n');
}

fn raw_geometry(out: &mut String, layer: &mlt::Layer, feature: &mlt::Feature) {
    let g = feature.get_geometry();
    let triangles = layer.get_triangles(feature);
    let _ = write!(out, "g {} t {}", g.type_() as u8, triangles.len());
    for i in triangles {
        let _ = write!(out, " {i}");
    }
    out.push('\n');
    match &g.shape {
        Shape::Point(p) => coords(out, &vec![*p]),
        Shape::MultiPoint(c) | Shape::LineString(c) => coords(out, c),
        Shape::Polygon(rings) | Shape::MultiLineString(rings) => {
            for r in rings {
                coords(out, r);
            }
        }
        Shape::MultiPolygon(polygons) => {
            for p in polygons {
                let _ = writeln!(out, "poly {}", p.len());
                for r in p {
                    coords(out, r);
                }
            }
        }
    }
}

/// reference.cpp `dumpRaw`.
fn dump_raw(out: &mut String, data: &[u8], fast_pfor: bool) {
    let tile = match MapLibreTile::decode(data, fast_pfor) {
        Ok(tile) => tile,
        Err(e) => {
            exception(out, "raw", &e);
            return;
        }
    };
    let _ = writeln!(out, "rawlayers {}", tile.get_layers().len());
    for layer in tile.get_layers() {
        out.push_str("rl ");
        hex(out, layer.get_name().as_bytes());
        let _ = writeln!(out, " {} {}", layer.get_extent(), layer.get_features().len());
        let mut columns: Vec<_> = layer.get_properties().iter().collect();
        columns.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
        for (key, column) in &columns {
            out.push_str("col ");
            hex(out, key.as_bytes());
            let type_ = match column.get_type() {
                mlt::ScalarType::Boolean => 0,
                mlt::ScalarType::Int32 => 3,
                mlt::ScalarType::UInt32 => 4,
                mlt::ScalarType::Int64 => 5,
                mlt::ScalarType::UInt64 => 6,
                mlt::ScalarType::Float => 7,
                mlt::ScalarType::Double => 8,
                mlt::ScalarType::String => 9,
            };
            let _ = writeln!(out, " {type_}");
        }
        for feature in layer.get_features() {
            out.push_str("rf ");
            match feature.get_id() {
                Some(id) => {
                    let _ = write!(out, "u{id}");
                }
                None => out.push('n'),
            }
            let _ = writeln!(out, " {}", feature.get_index());
            raw_geometry(out, layer, feature);
            for (c, (_, column)) in columns.iter().enumerate() {
                match column.get_property(feature.get_index()) {
                    Ok(p) => {
                        let _ = write!(out, "rp {c} ");
                        match p {
                            Some(p) => property(out, &p),
                            None => out.push_str("none"),
                        }
                        out.push('\n');
                    }
                    Err(e) => exception(out, &format!("rp {c}"), &e),
                }
            }
        }
    }
}

fn rings(out: &mut String, g: &GeometryCollection) {
    let _ = writeln!(out, "{}", g.len());
    for r in g.iter() {
        let _ = write!(out, "r {}", r.len());
        for p in r {
            let _ = write!(out, " {} {}", p.x, p.y);
        }
        out.push('\n');
    }
    let _ = write!(out, "tri {}", g.get_triangles().len());
    for i in g.get_triangles() {
        let _ = write!(out, " {i}");
    }
    out.push('\n');
}

const EXTRA_KEYS: [&str; 6] = ["name", "type", "class", "id", "zzz-missing", ""];

fn dump_feature(out: &mut String, layer: &mln_tile::VectorMLTTileLayer, i: usize) {
    let _ = writeln!(out, "f {i}");
    let feature = layer.feature(i);
    let _ = writeln!(out, "type {}", feature.get_type() as u8);
    out.push_str("id ");
    identifier(out, &feature.get_id());
    out.push('\n');

    let mut keys: Vec<String> = Vec::new();
    match feature.try_get_properties() {
        Ok(props) => {
            let _ = writeln!(out, "props {}", props.len());
            for (k, v) in props {
                out.push_str("p ");
                hex(out, k.as_bytes());
                out.push(' ');
                value(out, v);
                out.push('\n');
                keys.push(k.clone());
            }
        }
        Err(e) => exception(out, "props", &e),
    }
    keys.extend(EXTRA_KEYS.iter().map(|k| k.to_string()));
    for k in &keys {
        match feature.try_get_value(k) {
            Ok(v) => {
                out.push_str("v ");
                hex(out, k.as_bytes());
                out.push(' ');
                match v {
                    Some(v) => value(out, &v),
                    None => out.push_str("none"),
                }
                out.push('\n');
            }
            Err(e) => {
                let mut prefix = String::from("v ");
                hex(&mut prefix, k.as_bytes());
                exception(out, &prefix, &e);
            }
        }
    }

    out.push_str("geom ");
    rings(out, feature.get_geometries());
}

/// reference.cpp `mltDump`, from the port.
pub fn rust_dump(data: &[u8], fast_pfor: bool) -> String {
    let mut out = String::new();
    let tile = VectorMLTTileData::new(Arc::new(data.to_vec()), fast_pfor);

    let names = logged(&mut out, || tile.layer_names());
    let _ = writeln!(out, "names {}", names.len());
    for name in &names {
        out.push_str("name ");
        hex(&mut out, name.as_bytes());
        out.push('\n');
    }

    for name in &names {
        out.push_str("layer ");
        hex(&mut out, name.as_bytes());
        out.push('\n');
        let layer = logged(&mut out, || tile.get_mlt_layer(name));
        let Some(layer) = layer else {
            out.push_str("nolayer\n");
            continue;
        };
        out.push_str("info ");
        hex(&mut out, layer.get_name().as_bytes());
        let _ = writeln!(out, " {}", layer.feature_count());
        for i in 0..layer.feature_count() {
            dump_feature(&mut out, &layer, i);
        }
    }

    let missing = logged(&mut out, || tile.get_layer("zzz-missing-layer").is_some());
    out.push_str(if missing { "missing found\n" } else { "missing none\n" });

    let copy = tile.clone_data();
    let found = names.first().is_some_and(|name| copy.get_layer(name).is_some());
    let _ = writeln!(out, "copy {}", u8::from(found));

    dump_raw(&mut out, data, fast_pfor);
    out
}
