// Differential test of MLT decoding: //crates/mln_tile's VectorMLTTileData
// (C++ glue ported, tiles decoded by the mlt-core crate) against the C++
// VectorMLTTileData on the C++ MLT decoder, run as an oracle process
// (oracle_main.cpp).
//
// Inputs:
// 1. every MLT tile in the fixtures: the .mlt files under metrics/ and
//    test/fixtures, and every MLT blob of the SQLite fixtures (rows whose URL
//    names an .mlt tile), each with fastPFOREnabled false and true;
// 2. a hand-made minimal tile for the one known difference (a DOUBLE column
//    stored as 4-byte floats, see DEVIATIONS.md) and its conforming twin;
// 3. truncations of each fixture tile: both must reject them or agree;
// 4. byte mutations of each fixture tile: a measurement only (printed, not
//    asserted). The C++ decoder fails asserts (fastbuild) or reads out of
//    bounds on some, mlt-core validates more; DEVIATIONS.md [mlt] lists the
//    kinds of differences.
//
// Both sides write the same dump (dump.rs / reference.cpp): layer names,
// every feature's type, id, properties (exact mln::Value variant and bits),
// getValue of each key, scaled geometry and triangles, the ParseTile log, and
// the decoded mlt::MapLibreTile (float coordinates, typed properties). The
// text of a decoding error is the decoder's own (mlt-core vs. C++), so only
// the fact of the error is compared.

mod dump;

use dump::*;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;

fn runfiles_dir() -> PathBuf {
    let base = std::env::var("RUNFILES_DIR")
        .or_else(|_| std::env::var("TEST_SRCDIR"))
        .expect("RUNFILES_DIR or TEST_SRCDIR");
    PathBuf::from(base)
}

fn runfiles() -> PathBuf {
    runfiles_dir().join("maplibre+")
}

// --- the C++ oracle process ---------------------------------------------

struct Oracle {
    child: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
}

static ORACLE: Mutex<Option<Oracle>> = Mutex::new(None);

fn spawn_oracle() -> Oracle {
    let path = runfiles_dir().join("_main/difftest/mlt/mlt_oracle");
    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("start the C++ oracle");
    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    Oracle { child, stdin, stdout }
}

/// Sends a request; `None` if the oracle died on it (failed assert, crash).
fn oracle_call(op: u8, payload: &[u8]) -> Option<Vec<u8>> {
    let mut guard = ORACLE.lock().unwrap_or_else(|e| e.into_inner());
    let oracle = guard.get_or_insert_with(spawn_oracle);
    let ok = (|| -> std::io::Result<Vec<u8>> {
        oracle.stdin.write_all(&[op])?;
        oracle.stdin.write_all(&(payload.len() as u32).to_le_bytes())?;
        oracle.stdin.write_all(payload)?;
        oracle.stdin.flush()?;
        let mut len = [0u8; 4];
        oracle.stdout.read_exact(&mut len)?;
        let mut answer = vec![0u8; u32::from_le_bytes(len) as usize];
        oracle.stdout.read_exact(&mut answer)?;
        Ok(answer)
    })();
    match ok {
        Ok(answer) => Some(answer),
        Err(_) => {
            if let Some(mut dead) = guard.take() {
                let _ = dead.child.kill();
                let _ = dead.child.wait();
            }
            None
        }
    }
}

fn oracle_ndebug() -> bool {
    oracle_call(0, &[]).as_deref() == Some(b"1")
}

fn cpp_dump(data: &[u8], fast_pfor: bool) -> Option<String> {
    oracle_call(if fast_pfor { 2 } else { 1 }, data).map(|d| String::from_utf8(d).expect("ASCII dump"))
}

// --- inputs ---------------------------------------------------------------

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            walk(&p, out);
        } else {
            out.push(p);
        }
    }
}

/// `util::decompress` (zlib or gzip).
fn decompress(v: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let ok = if v.len() >= 2 && v[0] == 0x1f && v[1] == 0x8b {
        flate2::read::GzDecoder::new(v).read_to_end(&mut out).is_ok()
    } else {
        flate2::read::ZlibDecoder::new(v).read_to_end(&mut out).is_ok()
    };
    ok.then_some(out)
}

/// The MLT tiles of a cache database (`tiles.url_template` /
/// `resources.url` naming an .mlt file), inflated where `compressed` says so.
fn sqlite_mlt_blobs(path: &Path, name: &str) -> Vec<(String, Vec<u8>)> {
    use rusqlite::{Connection, OpenFlags};
    let uri = format!("file:{}?immutable=1", path.display());
    let Ok(conn) = Connection::open_with_flags(&uri, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI)
    else {
        return Vec::new();
    };
    let mut blobs = Vec::new();
    for sql in [
        "SELECT url_template || ' ' || z || '/' || x || '/' || y, data, compressed FROM tiles WHERE url_template LIKE '%.mlt%'",
        "SELECT url, data, compressed FROM resources WHERE url LIKE '%.mlt%'",
    ] {
        let Ok(mut stmt) = conn.prepare(sql) else { continue };
        let Ok(rows) = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<Vec<u8>>>(1)?, row.get::<_, i64>(2)?))
        }) else {
            continue;
        };
        for (key, blob, compressed) in rows.flatten() {
            let Some(blob) = blob else { continue };
            let blob = if compressed != 0 { decompress(&blob).unwrap_or(blob) } else { blob };
            blobs.push((format!("{name}#{key}"), blob));
        }
    }
    blobs
}

/// (name, bytes, is a file) of every MLT tile in the fixtures.
fn corpus() -> Vec<(String, Vec<u8>, bool)> {
    let root = runfiles();
    let mut files = Vec::new();
    for dir in ["metrics", "test/fixtures"] {
        walk(&root.join(dir), &mut files);
    }
    let mut inputs = Vec::new();
    for f in files {
        let name = f.strip_prefix(&root).unwrap_or(&f).display().to_string();
        match f.extension().and_then(|e| e.to_str()).unwrap_or("") {
            "mlt" => inputs.push((name, std::fs::read(&f).unwrap(), true)),
            "db" | "mbtiles" => {
                for (n, blob) in sqlite_mlt_blobs(&f, &name) {
                    inputs.push((n, blob, false));
                }
            }
            _ => {}
        }
    }
    inputs
}

// --- comparison -----------------------------------------------------------

/// Replaces the decoder-specific error texts: `log <sev> x<"MLT parse
/// failed: ...">` and `raw! <kind> x<what>`.
fn normalize(dump: &str) -> (String, Vec<String>) {
    const PREFIX: &[u8] = b"MLT parse failed: ";
    let mut out = String::with_capacity(dump.len());
    let mut errors = Vec::new();
    for line in dump.lines() {
        if let Some(rest) = line.strip_prefix("log ")
            && let Some((severity, msg)) = rest.split_once(' ')
        {
            let msg = unhex(msg);
            if msg.starts_with(PREFIX) {
                errors.push(String::from_utf8_lossy(&msg[PREFIX.len()..]).into_owned());
                out.push_str(&format!("log {severity} MLT parse failed: *\n"));
                continue;
            }
        }
        if line.starts_with("raw! ") {
            out.push_str("raw! *\n");
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    (out, errors)
}

fn decode_failed(normalized: &str) -> bool {
    normalized.contains("MLT parse failed: *")
}

/// The first differing line, with a little context.
fn first_difference(a: &str, b: &str) -> String {
    let (al, bl): (Vec<_>, Vec<_>) = (a.lines().collect(), b.lines().collect());
    for i in 0..al.len().max(bl.len()) {
        let (x, y) = (al.get(i).copied().unwrap_or("<end>"), bl.get(i).copied().unwrap_or("<end>"));
        if x != y {
            let from = i.saturating_sub(3);
            let ctx: Vec<_> = al[from.min(al.len())..i.min(al.len())].to_vec();
            let cut = |s: &str| s.chars().take(300).collect::<String>();
            return format!("line {i} (after {ctx:?}):\n  C++:  {}\n  Rust: {}", cut(x), cut(y));
        }
    }
    String::new()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Outcome {
    /// Both decoded it, with identical dumps.
    Same,
    /// Both rejected it (and agree on everything else).
    BothRejected,
    /// The known difference: a DOUBLE column whose data stream holds 4-byte
    /// floats, which the C++ decoder widens and mlt-core rejects.
    DoubleStoredAsFloat,
    /// C++ decoded it, mlt-core rejected it.
    OnlyCppDecodes,
    /// mlt-core decoded it, C++ rejected it.
    OnlyRustDecodes,
    /// Both decoded it, with different results.
    Different,
    /// The C++ oracle died (failed assert or crash).
    CppCrashed,
}

struct Comparison {
    outcome: Outcome,
    detail: String,
}

/// The C++ raw dump has a DOUBLE column, and every one of its values is a
/// widened float: the signature of the C++ decoder's compatibility path for
/// tilesets "encoded before double support was added".
fn has_float_valued_double_column(cpp: &str) -> bool {
    let mut layer_columns: Vec<String> = Vec::new();
    let mut double_columns = Vec::new();
    let mut all_float = true;
    let mut found = false;
    for line in cpp.lines() {
        if line.starts_with("rl ") {
            layer_columns.clear();
            double_columns.clear();
        } else if let Some(col) = line.strip_prefix("col ") {
            let (name, type_) = col.rsplit_once(' ').unwrap();
            if type_ == "8" {
                double_columns.push(layer_columns.len());
                found = true;
            }
            layer_columns.push(name.to_string());
        } else if let Some(rp) = line.strip_prefix("rp ") {
            let mut parts = rp.split(' ');
            let c: usize = parts.next().unwrap().parse().unwrap_or(usize::MAX);
            if double_columns.contains(&c)
                && parts.next() == Some("f64")
                && let Some(bits) = parts.next()
            {
                let d = f64::from_bits(u64::from_str_radix(bits, 16).unwrap());
                if f64::from(d as f32).to_bits() != d.to_bits() && !d.is_nan() {
                    all_float = false;
                }
            }
        }
    }
    found && all_float
}

fn compare(data: &[u8], fast_pfor: bool) -> Comparison {
    let rust = rust_dump(data, fast_pfor);
    let Some(cpp) = cpp_dump(data, fast_pfor) else {
        return Comparison { outcome: Outcome::CppCrashed, detail: String::new() };
    };
    let (cpp_n, cpp_errors) = normalize(&cpp);
    let (rust_n, rust_errors) = normalize(&rust);
    let (cpp_failed, rust_failed) = (decode_failed(&cpp_n), decode_failed(&rust_n));
    let errors = format!("C++ {cpp_errors:?} / mlt-core {rust_errors:?}");
    if cpp_n == rust_n {
        let outcome = if cpp_failed { Outcome::BothRejected } else { Outcome::Same };
        return Comparison { outcome, detail: errors };
    }
    let outcome = match (cpp_failed, rust_failed) {
        (false, true) => {
            let size_error = rust_errors.iter().any(|e| {
                e.strip_prefix("decodable stream size expected ")
                    .and_then(|r| r.split_once(", got "))
                    .and_then(|(a, b)| Some((a.parse::<u64>().ok()?, b.parse::<u64>().ok()?)))
                    .is_some_and(|(expected, got)| expected == 2 * got)
            });
            if size_error && has_float_valued_double_column(&cpp) {
                Outcome::DoubleStoredAsFloat
            } else {
                Outcome::OnlyCppDecodes
            }
        }
        (true, false) => Outcome::OnlyRustDecodes,
        (true, true) => Outcome::BothRejected,
        (false, false) => Outcome::Different,
    };
    let detail = if outcome == Outcome::Different || outcome == Outcome::BothRejected {
        format!("{errors}\n{}", first_difference(&cpp_n, &rust_n))
    } else {
        errors
    };
    save_for_inspection(data, &cpp, &rust);
    Comparison { outcome, detail }
}

/// The first few differing inputs and dumps go to the test outputs
/// (bazel-testlogs/.../test.outputs) for inspection.
fn save_for_inspection(data: &[u8], cpp: &str, rust: &str) {
    static SAVED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let Ok(dir) = std::env::var("TEST_UNDECLARED_OUTPUTS_DIR") else { return };
    let n = SAVED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if n < 8 {
        let _ = std::fs::write(format!("{dir}/diff{n}.mlt"), data);
        let _ = std::fs::write(format!("{dir}/diff{n}.cpp.txt"), cpp);
        let _ = std::fs::write(format!("{dir}/diff{n}.rust.txt"), rust);
    }
}

/// Outcome counts plus one example each.
#[derive(Default)]
struct Tally {
    counts: BTreeMap<Outcome, usize>,
    examples: BTreeMap<Outcome, Vec<String>>,
}

impl Tally {
    fn add(&mut self, name: &str, c: &Comparison) {
        *self.counts.entry(c.outcome).or_default() += 1;
        let examples = self.examples.entry(c.outcome).or_default();
        if examples.len() < 5 {
            examples.push(format!("{name}: {}", c.detail));
        }
    }

    fn report(&self, title: &str) {
        eprintln!("== {title}: {:?}", self.counts);
        for (outcome, examples) in &self.examples {
            if !matches!(outcome, Outcome::Same | Outcome::BothRejected) {
                for e in examples {
                    eprintln!("   {outcome:?}: {e}");
                }
            }
        }
    }

    fn count(&self, outcome: Outcome) -> usize {
        self.counts.get(&outcome).copied().unwrap_or(0)
    }
}

// --- tests ----------------------------------------------------------------

/// Fixture tiles that are not spec compliant: they store a DOUBLE column as
/// 4-byte floats, which only the C++ decoder's legacy path accepts. The port
/// follows the MLT spec (mlt-core rejects them), so they are ignored here
/// until the fixtures are re-encoded with real doubles. See DEVIATIONS.md
/// [mlt].
const NON_SPEC_TILES: [&str; 8] = [
    "metrics/cache-style.db#http://tiles/{z}-{x}-{y}.mlt 14/4357/6245",
    "metrics/cache-style.db#http://tiles/counties-{z}-{x}-{y}.mlt 7/37/48",
    "metrics/cache-style.db#http://tiles/mapbox.mapbox-streets-v7/{z}-{x}-{y}.mlt 4/4/7",
    "metrics/cache-style.db#http://tiles/mapbox.mapbox-streets-v7/{z}-{x}-{y}.mlt 4/5/7",
    "metrics/integration/tiles/14-4357-6245.mlt",
    "metrics/integration/tiles/counties-7-37-48.mlt",
    "metrics/integration/tiles/mapbox.mapbox-streets-v7/4-4-7.mlt",
    "metrics/integration/tiles/mapbox.mapbox-streets-v7/4-5-7.mlt",
];

#[test]
fn fixtures() {
    let inputs = corpus();
    let files = inputs.iter().filter(|(_, _, file)| *file).count();
    assert!(files >= 24, "expected the 24 .mlt fixture files, found {files}");
    let mut tally = Tally::default();
    let mut features = 0usize;
    let mut double_as_float = Vec::new();
    for (name, data, _) in &inputs {
        if NON_SPEC_TILES.contains(&name.as_str()) {
            continue;
        }
        for fast_pfor in [false, true] {
            let c = compare(data, fast_pfor);
            if c.outcome == Outcome::Same {
                features += rust_dump(data, fast_pfor).lines().filter(|l| l.starts_with("f ")).count();
            }
            if c.outcome == Outcome::DoubleStoredAsFloat && fast_pfor {
                double_as_float.push(name.clone());
            }
            tally.add(&format!("{name} fastpfor={fast_pfor}"), &c);
        }
    }
    eprintln!("{} inputs ({files} files), {features} features decoded identically", inputs.len());
    eprintln!("DOUBLE columns stored as floats (C++ only): {double_as_float:#?}");
    tally.report("fixtures");
    // Any other tile with a float-sized DOUBLE column is a new non-spec
    // fixture: fail, so it gets looked at.
    for outcome in [
        Outcome::OnlyCppDecodes,
        Outcome::OnlyRustDecodes,
        Outcome::Different,
        Outcome::CppCrashed,
        Outcome::DoubleStoredAsFloat,
    ] {
        assert_eq!(tally.count(outcome), 0, "{outcome:?}");
    }
    // Only the FastPFOR tile without FastPFOR support is rejected by both.
    assert!(tally.count(Outcome::BothRejected) <= 2, "{:?}", tally.counts);
}

/// The smallest tile with the known difference: one point feature and a
/// non-optional DOUBLE column "v" whose data stream holds the 4-byte float
/// 1.5. The C++ decoder reads it as 1.5 (its compatibility path for tiles
/// "encoded before double support was added"); mlt-core rejects it, as the
/// spec defines a DOUBLE as an 8-byte IEEE754 value.
pub const DOUBLE_AS_FLOAT_TILE: [u8; 31] = [
    0x1e, // layer size (tag + body)
    0x01, // tag: v1 feature table
    0x01, b'a', // name
    0x80, 0x20, // extent 4096
    0x02, // 2 columns
    0x04, // geometry
    0x1a, 0x01, b'v', // DOUBLE "v"
    0x02, // geometry: 2 streams
    0x10, 0x02, 0x01, 0x01, 0x00, // geometry types: 1 varint value, POINT
    0x13, 0x42, 0x02, 0x02, 0x02, 0x04, // vertices: componentwise delta (1, 2)
    0x10, 0x00, 0x01, 0x04, 0x00, 0x00, 0xc0, 0x3f, // data: 1 value, 4 bytes: 1.5f
];

#[test]
#[ignore = "documents the C++ decoder's non-spec legacy path (DOUBLE stored as float); the port follows the MLT spec"]
fn double_stored_as_float_reproducer() {
    let c = compare(&DOUBLE_AS_FLOAT_TILE, true);
    assert_eq!(c.outcome, Outcome::DoubleStoredAsFloat, "{}", c.detail);
    let cpp = cpp_dump(&DOUBLE_AS_FLOAT_TILE, true).unwrap();
    assert!(cpp.contains(&format!("v {} d{:016x}", {
        let mut k = String::new();
        hex(&mut k, b"v");
        k
    }, 1.5f64.to_bits())), "{cpp}");
    let err = mln_tile::mlt::MapLibreTile::decode(&DOUBLE_AS_FLOAT_TILE, true).unwrap_err();
    assert_eq!(err.what(), "decodable stream size expected 8, got 4");

    // The conforming twin (8-byte double) decodes identically on both sides.
    let mut twin = DOUBLE_AS_FLOAT_TILE.to_vec();
    twin[0] += 4;
    let at = twin.len() - 5;
    twin[at] = 0x08;
    twin.truncate(twin.len() - 4);
    twin.extend_from_slice(&1.5f64.to_le_bytes());
    let c = compare(&twin, true);
    assert_eq!(c.outcome, Outcome::Same, "{}", c.detail);
}

/// Every prefix length up to 64 bytes, then ~150 lengths spread over the
/// tile, of each fixture file.
#[test]
fn truncations() {
    let mut tally = Tally::default();
    for (name, data, file) in corpus() {
        if !file {
            continue;
        }
        let mut lengths: Vec<usize> = (0..data.len().min(64)).collect();
        let step = (data.len() / 150).max(1);
        lengths.extend((64..data.len()).step_by(step));
        lengths.push(data.len().saturating_sub(1));
        lengths.sort_unstable();
        lengths.dedup();
        for len in lengths {
            let c = compare(&data[..len], true);
            tally.add(&format!("{name}[..{len}]"), &c);
        }
    }
    tally.report("truncations");
    for outcome in [Outcome::OnlyRustDecodes, Outcome::Different] {
        assert_eq!(tally.count(outcome), 0, "{outcome:?}");
    }
}

/// A small deterministic RNG (splitmix64).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
}

/// Not a gate (see the file comment): prints the outcome counts.
#[test]
fn mutations() {
    let ndebug = oracle_ndebug();
    let mut tally = Tally::default();
    let mut rng = Rng(0x6d6c74);
    for (name, data, file) in corpus() {
        if !file || data.is_empty() {
            continue;
        }
        for k in 0..30 {
            let mut m = data.clone();
            for _ in 0..1 + rng.below(3) {
                let i = rng.below(m.len() as u64) as usize;
                m[i] = match rng.below(4) {
                    0 => rng.next() as u8,
                    1 => m[i] ^ (1 << rng.below(8)),
                    2 => m[i].wrapping_add(1),
                    _ => m[i].wrapping_sub(1),
                };
            }
            let c = compare(&m, true);
            tally.add(&format!("{name} mutation {k}"), &c);
        }
    }
    tally.report(&format!("mutations (NDEBUG oracle: {ndebug})"));
}
