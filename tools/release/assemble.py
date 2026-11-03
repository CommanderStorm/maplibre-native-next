#!/usr/bin/env python3
"""Builds the crates.io release of maplibre-native-next (see RELEASING.md).

Development is multi-crate (Bazel rust_library targets); crates.io gets one
crate, because it rate-limits new crates and consumers want one dependency.
Run with `python3 -I tools/release/assemble.py <command>`:

  assemble [--cpp-repo DIR] [--out DIR]
      Writes a Cargo workspace (default: target/release-crate) with
      - maplibre-native-next: every rust_library under crates/ and vendor/ as
        a public top-level module named after its crate (mln_util, mln_map,
        earcut, ...), with its paths rewritten (see rewrite()), its
        third-party dependencies from MODULE.bazel and MODULE.bazel.lock, and
        the generated sources (the WGSL from the C++ headers in --cpp-repo,
        default ../maplibre-native);
      - one package per patched third-party crate (crate.annotation patches
        in MODULE.bazel), named maplibre-native-next-<crate>: the crates.io
        source with the patches applied, keeping the library name.
  status [--out DIR] [--target-dir DIR]
      After `cargo package`, prints the packages crates.io does not have yet
      (in publish order). Fails if crates.io has a version with different
      contents: bump the version in tools/release/package.toml.
  version
      Prints the version of the merged crate.
"""

import argparse
import ast
import glob
import hashlib
import io
import json
import re
import shutil
import subprocess
import sys
import tarfile
import tomllib
import urllib.error
import urllib.request
from dataclasses import dataclass, field
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
CONFIG = REPO / "tools/release/package.toml"
LIBRARY_ROOTS = ("crates", "vendor")
CRATES_REPO = "next_crates"  # crate.from_specs(name = ...) in MODULE.bazel
USER_AGENT = "maplibre-native-next-release (https://github.com/louwers/maplibre-native-next)"

# C++ filegroups that generated sources read, as globs in the C++ checkout.
CPP_FILEGROUPS = {
    "@maplibre//:webgpu-shader-headers": "include/mln/shaders/webgpu/*.hpp",
}

# Inner attributes a module may carry; any other inner attribute of a lib.rs
# is crate-level and moves to the merged root.
MODULE_INNER_ATTRS = {"allow", "warn", "deny", "forbid", "expect", "doc", "cfg", "cfg_attr"}
ROOT_ONLY_ATTRS_REJECTED = {"no_std", "no_main", "crate_type", "crate_name", "no_builtins"}


def fail(msg):
    sys.exit(f"assemble.py: error: {msg}")


# --------------------------------------------------------------------------
# Bazel files (Starlark parses as Python).


def calls(path):
    tree = ast.parse(path.read_text(), str(path))
    for node in ast.walk(tree):
        if isinstance(node, ast.Call):
            f = node.func
            if isinstance(f, ast.Name):
                yield f.id, node
            elif isinstance(f, ast.Attribute) and isinstance(f.value, ast.Name):
                yield f"{f.value.id}.{f.attr}", node


def top_level_constants(path):
    out = {}
    for node in ast.parse(path.read_text(), str(path)).body:
        if isinstance(node, ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
            try:
                out[node.targets[0].id] = ast.literal_eval(node.value)
            except ValueError:
                pass
    return out


def kwargs(call, constants=None):
    out = {}
    for kw in call.keywords:
        if isinstance(kw.value, ast.Name) and constants and kw.value.id in constants:
            out[kw.arg] = constants[kw.value.id]
            continue
        try:
            out[kw.arg] = ast.literal_eval(kw.value)
        except ValueError:
            out[kw.arg] = kw.value
    return out


@dataclass
class Generated:
    out: str  # path relative to the package, e.g. src/shaders/wgsl_sources.rs
    tool_src: str  # rust_binary source, relative to the package
    inputs: list  # C++ filegroup labels


@dataclass
class Library:
    name: str  # crate name = module name in the merged crate
    package: str  # Bazel package, e.g. crates/mln_util
    deps: list = field(default_factory=list)  # in-repo libraries
    test_deps: set = field(default_factory=set)  # in-repo, from rust_test(crate = ...)
    external: list = field(default_factory=list)  # @next_crates names
    generated: list = field(default_factory=list)


def parse_label(label, package):
    if label.startswith(":"):
        return package, label[1:]
    if not label.startswith("//"):
        return None
    pkg, _, target = label[2:].partition(":")
    return pkg, target or pkg.rsplit("/", 1)[-1]


def srcs_generated_labels(node, package):
    """The non-glob labels of `srcs = glob(["src/**/*.rs"]) (+ [...])`."""
    if isinstance(node, ast.BinOp) and isinstance(node.op, ast.Add):
        return srcs_generated_labels(node.left, package) + srcs_generated_labels(node.right, package)
    if isinstance(node, ast.Call) and getattr(node.func, "id", None) == "glob":
        if ast.literal_eval(node.args[0]) != ["src/**/*.rs"] or node.keywords:
            fail(f"{package}: library srcs must be glob([\"src/**/*.rs\"]), found {ast.unparse(node)}")
        return []
    if isinstance(node, ast.List):
        return ast.literal_eval(node)
    fail(f"{package}: unsupported srcs {ast.unparse(node)}")


def load_libraries():
    libs = {}  # (package, target) -> Library
    tests = []
    for root in LIBRARY_ROOTS:
        for build in sorted((REPO / root).glob("*/BUILD.bazel")):
            package = str(build.parent.relative_to(REPO))
            constants = top_level_constants(build)
            rules = {}
            for kind, call in calls(build):
                kw = kwargs(call, constants)
                if "name" in kw:
                    rules[kw["name"]] = (kind, kw)
            for target, (kind, kw) in rules.items():
                if kind == "rust_test" and isinstance(kw.get("crate"), str):
                    tests.append((package, kw))
                if kind != "rust_library" or kw.get("testonly"):
                    continue
                for attr in ("crate_root", "crate_name", "rustc_flags", "crate_features", "edition", "proc_macro_deps", "compile_data", "rustc_env"):
                    if attr in kw:
                        fail(f"//{package}:{target}: {attr} is not supported by the release assembly")
                lib = Library(name=target.replace("-", "_"), package=package)
                for label in srcs_generated_labels(kw["srcs"], package):
                    gen_pkg, gen_target = parse_label(label, package)
                    gen_kind, gen_kw = rules.get(gen_target, (None, None))
                    if gen_pkg != package or gen_kind != "wgsl_sources":
                        fail(f"//{package}:{target}: unsupported generated source {label}")
                    tool_kind, tool_kw = rules[parse_label(gen_kw["tool"], package)[1]]
                    if tool_kind != "rust_binary" or len(tool_kw["srcs"]) != 1:
                        fail(f"//{package}:{target}: unsupported generator {gen_kw['tool']}")
                    lib.generated.append(Generated(gen_kw["out"], tool_kw["srcs"][0], gen_kw["srcs"]))
                for dep in kw.get("deps", []):
                    if dep.startswith(f"@{CRATES_REPO}//:"):
                        lib.external.append(dep.split(":", 1)[1])
                    else:
                        lib.deps.append(dep)
                libs[(package, target)] = lib
    by_label = {key: lib.name for key, lib in libs.items()}
    for lib in libs.values():
        resolved = []
        for dep in lib.deps:
            key = parse_label(dep, lib.package)
            if key not in by_label:
                fail(f"//{lib.package}: dependency {dep} is not a library under {'/'.join(LIBRARY_ROOTS)}/")
            resolved.append(by_label[key])
        lib.deps = resolved
    for package, kw in tests:
        key = parse_label(kw["crate"], package)
        lib = libs.get(key)
        if lib:
            for dep in kw.get("deps", []):
                dkey = parse_label(dep, package)
                if dkey in by_label:
                    lib.test_deps.add(by_label[dkey])
    names = [lib.name for lib in libs.values()]
    if len(set(names)) != len(names):
        fail(f"duplicate crate names: {sorted(names)}")
    return sorted(libs.values(), key=lambda lib: lib.name)


def load_module():
    """MODULE.bazel: the crate.spec entries, the patched crates, the edition."""
    specs, patched, edition = {}, {}, None
    for kind, call in calls(REPO / "MODULE.bazel"):
        kw = kwargs(call)
        if kind == "crate.spec":
            specs[kw["package"]] = kw
        elif kind == "crate.annotation" and kw.get("patches"):
            patched[kw["crate"]] = kw
        elif kind == "rust.toolchain":
            edition = kw.get("edition")
    return specs, patched, edition


def load_lock():
    lock = json.loads((REPO / "MODULE.bazel.lock").read_text())
    exts = [v for k, v in lock["moduleExtensions"].items() if k.endswith("crate_universe:extensions.bzl%crate")]
    if len(exts) != 1:
        fail("MODULE.bazel.lock: expected one crate_universe extension")
    return exts[0]["general"]["generatedRepoSpecs"]


def resolved_crate(lock, name):
    """(version, repo spec attributes) Bazel resolved for @next_crates//:name."""
    contents = lock[CRATES_REPO]["attributes"]["contents"]
    alias = contents.get(f"{name}/BUILD.bazel")
    m = alias and re.search(rf'actual = "@{CRATES_REPO}__{re.escape(name)}-([^/"]+)//', alias)
    if not m:
        fail(f"MODULE.bazel.lock: no resolved version for {name}; run bazel mod deps")
    version = m.group(1)
    return version, lock[f"{CRATES_REPO}__{name}-{version}"]["attributes"]


# --------------------------------------------------------------------------
# Rust source rewriting.

# Keywords after which `::name` is an absolute path (`crate`, `self` and
# `super` are path segments, so not here).
KEYWORDS = {
    "as", "async", "await", "box", "break", "const", "continue", "dyn", "else", "enum", "extern",
    "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
    "return", "static", "struct", "trait", "type", "union", "unsafe", "use", "where", "while",
    "yield",
}
RAW_STR = re.compile(r"(?:br|cr|r)(#*)\"")
IDENT = re.compile(r"(?:r#)?[^\W\d]\w*")
NUMBER = re.compile(r"\d\w*(?:\.\d\w*)?")


@dataclass
class Tok:
    kind: str  # ident, punct, lit, lifetime, comment, ws
    start: int
    end: int
    text: str


def lex(src):
    toks, i, n = [], 0, len(src)
    while i < n:
        c = src[i]
        if c.isspace():
            j = i + 1
            while j < n and src[j].isspace():
                j += 1
            kind = "ws"
        elif src.startswith("//", i):
            j = src.find("\n", i)
            j = n if j < 0 else j
            kind = "comment"
        elif src.startswith("/*", i):
            depth, j = 1, i + 2
            while depth:
                if src.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif src.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                elif j >= n:
                    fail("unterminated block comment")
                else:
                    j += 1
            kind = "comment"
        elif (m := RAW_STR.match(src, i)) :
            close = '"' + m.group(1)
            j = src.index(close, m.end()) + len(close)
            kind = "lit"
        elif c == '"' or src.startswith(('b"', 'c"'), i):
            j = src.index('"', i) + 1
            while src[j] != '"':
                j += 2 if src[j] == "\\" else 1
            j += 1
            kind = "lit"
        elif c == "'" or src.startswith("b'", i):
            q = src.index("'", i)
            if src[q + 1] == "\\":
                j = src.index("'", q + 3) + 1
                kind = "lit"
            elif q + 2 < n and src[q + 2] == "'":
                j = q + 3
                kind = "lit"
            elif (m := IDENT.match(src, q + 1)):
                j = m.end()
                kind = "lifetime"
            else:
                j = q + 1
                kind = "punct"
        elif (m := IDENT.match(src, i)):
            j = m.end()
            kind = "ident"
        elif c.isdigit():
            j = NUMBER.match(src, i).end()
            kind = "lit"
        elif src.startswith("::", i):
            j = i + 2
            kind = "punct"
        else:
            j = i + 1
            kind = "punct"
        toks.append(Tok(kind, i, j, src[i:j]))
        i = j
    return toks


def significant(src):
    return [t for t in lex(src) if t.kind not in ("ws", "comment")]


def is_ident(t, text=None):
    return t is not None and t.kind == "ident" and (text is None or t.text == text)


def is_punct(t, text):
    return t is not None and t.kind == "punct" and t.text == text


class UseTrees:
    """Walks `use` declarations: root path starts and the names they bind."""

    def __init__(self, toks):
        self.toks = toks
        self.root_starts = {}  # token index of a root segment -> index of its leading `::` or None
        self.bindings = []  # (bound name, full path)
        i = 0
        while i < len(toks):
            t = toks[i]
            prev = toks[i - 1] if i else None
            nxt = toks[i + 1] if i + 1 < len(toks) else None
            if is_ident(t, "use") and not is_punct(prev, ".") and not is_punct(prev, "::") and not is_punct(nxt, "<"):
                i = self._walk(i + 1)
            else:
                i += 1

    def _walk(self, i):
        toks = self.toks
        prefixes = [[]]  # path prefix of each open brace group
        roots = [True]  # whether the group is the root of the tree
        path, expect_root, leading, alias_next = [], True, None, False
        while i < len(toks):
            t = toks[i]
            if alias_next:
                if t.text != "_":
                    self.bindings.append((t.text, path))
                alias_next, path = False, None
            elif is_punct(t, "::"):
                if expect_root and not path:
                    leading = i
            elif is_punct(t, "{"):
                prefixes.append(list(path or []))
                roots.append(expect_root and not path)
                path = list(prefixes[-1])
                expect_root = roots[-1]
            elif is_punct(t, ",") or is_punct(t, "}") or is_punct(t, ";"):
                if path:
                    name = path[-1] if path[-1] != "self" else (path[-2] if len(path) > 1 else None)
                    if name:
                        self.bindings.append((name, path))
                if is_punct(t, "}"):
                    prefixes.pop()
                    roots.pop()
                    path = None
                elif is_punct(t, ";"):
                    return i + 1
                else:
                    path = list(prefixes[-1])
                    expect_root = roots[-1]
                    leading = None
            elif is_ident(t, "as"):
                alias_next = True
            elif is_punct(t, "*"):
                path = None
            elif t.kind == "ident":
                if expect_root and not path:
                    self.root_starts[i] = leading
                    expect_root, leading = False, None
                path = (path or []) + [t.text]
            i += 1
        return i


def rewrite(src, module, crates):
    """Rewrites a source file of former crate `module` for the merged crate.

    - `crate::` and `$crate::` -> `crate::module::` / `$crate::module::`
      (`pub(crate)` stays; `pub(in crate::a)` becomes `pub(in crate::module::a)`).
    - Paths starting with a crate in `crates` (the crate's Bazel deps):
      `mln_util::x`, `::mln_util::x`, `use mln_util;`, `use {mln_util::x, ..}`
      -> `crate::mln_util::...`. Not when the file binds that name itself
      (`mod geojson;`, `use a::geojson;`), and not for `name::<` (a function).
    Returns the new source and the names the file shadows.
    """
    toks = significant(src)
    uses = UseTrees(toks)
    shadowed = {name for name, path in uses.bindings if name in crates and path not in ([name], [name, "self"])}
    for i, t in enumerate(toks[:-1]):
        if is_ident(t) and t.text in {"mod", "fn", "struct", "enum", "trait", "type", "union", "static", "const"}:
            if is_ident(toks[i + 1]) and toks[i + 1].text in crates:
                shadowed.add(toks[i + 1].text)
    edits = []  # (start, end, replacement)
    for i, t in enumerate(toks):
        prev = toks[i - 1] if i else None
        nxt = toks[i + 1] if i + 1 < len(toks) else None
        nxt2 = toks[i + 2] if i + 2 < len(toks) else None
        if is_ident(t, "crate") and is_punct(nxt, "::"):
            edits.append((nxt.end, nxt.end, f"{module}::"))
            continue
        if t.kind != "ident" or t.text not in crates:
            continue
        if i in uses.root_starts:
            leading = uses.root_starts[i]
            if leading is not None:
                edits.append((toks[leading].start, toks[leading].end, "crate::"))
            else:
                edits.append((t.start, t.start, "crate::"))
            continue
        if not is_punct(nxt, "::") or is_punct(nxt2, "<") or t.text in shadowed:
            continue
        if is_punct(prev, "."):
            continue
        if is_punct(prev, "$"):
            continue
        if is_punct(prev, "::"):
            before = toks[i - 2] if i >= 2 else None
            leading = not (is_punct(before, ">") or is_punct(before, ")") or (is_ident(before) and before.text not in KEYWORDS))
            if leading:
                edits.append((prev.start, prev.end, "crate::"))
            continue
        edits.append((t.start, t.start, "crate::"))
    out = src
    for start, end, text in sorted(edits, reverse=True):
        out = out[:start] + text + out[end:]
    return out, shadowed & set(crates), len(edits)


def exported_macros(src):
    """Names of the `#[macro_export] macro_rules!` in a file."""
    toks, names, pending = significant(src), [], False
    for i, t in enumerate(toks):
        if is_ident(t, "macro_export") and is_punct(toks[i - 1], "["):
            pending = True
        elif pending and is_ident(t, "macro_rules") and is_punct(toks[i + 1], "!"):
            names.append(toks[i + 2].text)
            pending = False
    return names


def split_inner_attrs(src):
    """Splits the leading inner attributes of a lib.rs off: (module-level
    source, crate-level attributes as (name, text))."""
    sig = significant(src)
    crate_attrs, removals, i = [], [], 0
    while i < len(sig):
        if not (is_punct(sig[i], "#") and i + 2 < len(sig) and is_punct(sig[i + 1], "!") and is_punct(sig[i + 2], "[")):
            break
        depth, j = 0, i + 2
        while True:
            if is_punct(sig[j], "["):
                depth += 1
            elif is_punct(sig[j], "]"):
                depth -= 1
                if not depth:
                    break
            j += 1
        name = sig[i + 3].text
        text = src[sig[i].start : sig[j].end]
        if name in ROOT_ONLY_ATTRS_REJECTED:
            fail(f"#![{name}] cannot be merged into one crate")
        is_tool_attr = is_punct(sig[i + 4], "::")
        if name not in MODULE_INNER_ATTRS and not is_tool_attr:
            crate_attrs.append((name, text))
            removals.append((sig[i].start, sig[j].end))
        i = j + 1
    for start, end in reversed(removals):
        src = src[:start] + src[end:]
    return src, crate_attrs


def merge_crate_attrs(attrs):
    out, limits = [], {}
    for name, text in attrs:
        if name in ("recursion_limit", "type_length_limit"):
            limits[name] = max(limits.get(name, 0), int(re.search(r'"(\d+)"', text).group(1)))
        elif text not in out:
            out.append(text)
    out += [f'#![{name} = "{value}"]' for name, value in sorted(limits.items())]
    return out


# --------------------------------------------------------------------------
# Output.


def toml_value(v):
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, (int, float)):
        return str(v)
    if isinstance(v, str):
        return json.dumps(v)
    if isinstance(v, list):
        return "[" + ", ".join(toml_value(x) for x in v) + "]"
    if isinstance(v, dict):
        return "{ " + ", ".join(f"{toml_key(k)} = {toml_value(x)}" for k, x in v.items()) + " }"
    raise TypeError(v)


def toml_key(k):
    return k if re.fullmatch(r"[A-Za-z0-9_-]+", k) else json.dumps(k)


def toml_tables(d, prefix=()):
    """TOML text for nested dicts: dicts become [tables], the rest inline."""
    lines = []
    scalars = [(k, v) for k, v in d.items() if not isinstance(v, dict)]
    tables = [(k, v) for k, v in d.items() if isinstance(v, dict)]
    if prefix and (scalars or not tables):
        lines.append("[" + ".".join(toml_key(k) for k in prefix) + "]")
    lines += [f"{toml_key(k)} = {toml_value(v)}" for k, v in scalars]
    if scalars:
        lines.append("")
    for k, v in tables:
        lines += toml_tables(v, prefix + (k,))
    return lines


def write(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def fetch(url):
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(req, timeout=60) as r:
        return r.read()


def build_fork(crate, spec_attrs, upstream_version, fork_cfg, package_name, out):
    """The crates.io source of `crate` with the MODULE.bazel patches."""
    name = f"{package_name}-{crate}"
    cache = out / "download" / f"{crate}-{upstream_version}.crate"
    if not cache.exists() or hashlib.sha256(cache.read_bytes()).hexdigest() != spec_attrs["sha256"]:
        data = fetch(spec_attrs["urls"][0])
        if hashlib.sha256(data).hexdigest() != spec_attrs["sha256"]:
            fail(f"{crate} {upstream_version}: checksum mismatch with MODULE.bazel.lock")
        cache.parent.mkdir(parents=True, exist_ok=True)
        cache.write_bytes(data)
    dest = out / name
    shutil.rmtree(dest, ignore_errors=True)
    prefix = spec_attrs["strip_prefix"] + "/"
    with tarfile.open(cache) as tar:
        for member in tar.getmembers():
            if not member.name.startswith(prefix) or not (member.isfile() or member.isdir()):
                continue
            member.name = member.name[len(prefix) :]
            if member.name in ("Cargo.toml.orig", "Cargo.lock", ".cargo_vcs_info.json") or not member.name:
                continue
            tar.extract(member, dest, filter="data")
    patches = []
    for label in spec_attrs.get("patches", []):
        pkg, target = parse_label(label.removeprefix("@@"), "")
        patch = REPO / pkg / target
        patches.append(patch)
        cmd = ["patch", "--batch", "--no-backup-if-mismatch", "-d", str(dest), "-i", str(patch)]
        subprocess.run(cmd + spec_attrs.get("patch_args", []), check=True, stdout=subprocess.DEVNULL)
        (dest / "patches").mkdir(exist_ok=True)
        shutil.copy(patch, dest / "patches" / patch.name)
    manifest = tomllib.loads((dest / "Cargo.toml").read_text())
    pkg = manifest["package"]
    if pkg["name"] != crate or pkg["version"] != upstream_version:
        fail(f"{cache}: unexpected package {pkg['name']} {pkg['version']}")
    upstream_repo = pkg.get("repository")
    pkg["name"] = name
    pkg["version"] = fork_cfg["version"]
    pkg["description"] = f"{crate} {upstream_version} with the patches maplibre-native-next needs (in patches/)"
    pkg["repository"] = "https://github.com/louwers/maplibre-native-next"
    if upstream_repo:
        pkg["homepage"] = upstream_repo
    for key in ("keywords", "categories", "documentation"):
        pkg.pop(key, None)
    manifest.setdefault("lib", {}).setdefault("name", crate.replace("-", "_"))
    manifest.pop("dev-dependencies", None)
    for target in manifest.get("target", {}).values():
        target.pop("dev-dependencies", None)
    header = (
        f"# Generated by tools/release/assemble.py: {crate} {upstream_version} from crates.io\n"
        f"# plus {', '.join(p.name for p in patches)}, published as {name}.\n\n"
    )
    write(dest / "Cargo.toml", header + "\n".join(toml_tables(manifest)).rstrip() + "\n")
    readme = dest / pkg.get("readme", "README.md")
    note = (
        f"This is **{name}**: {crate} {upstream_version} with the patches in `patches/`, "
        f"which [maplibre-native-next](https://github.com/louwers/maplibre-native-next) "
        f"depends on. Other projects should use the upstream crate.\n\n"
    )
    if readme.is_file():
        readme.write_text(note + readme.read_text())
    return name, fork_cfg["version"]


def generate_wgsl(gen, lib_dir, dest, cpp_repo, tools_dir, edition):
    tool = tools_dir / Path(gen.tool_src).stem
    tools_dir.mkdir(parents=True, exist_ok=True)
    subprocess.run(["rustc", "--edition", edition, "-O", "-o", str(tool), str(lib_dir / gen.tool_src)], check=True)
    headers = []
    for label in gen.inputs:
        if label not in CPP_FILEGROUPS:
            fail(f"unknown generator input {label}")
        headers += sorted(glob.glob(str(cpp_repo / CPP_FILEGROUPS[label])))
    if not headers:
        fail(f"no C++ headers under {cpp_repo}; pass --cpp-repo (a maplibre-native checkout)")
    dest.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run([str(tool), str(dest), *headers], check=True)


def assemble(args):
    config = tomllib.loads(CONFIG.read_text())
    package = config["package"]
    out = Path(args.out).resolve()
    cpp_repo = Path(args.cpp_repo).resolve()
    libs = load_libraries()
    specs, patched, edition = load_module()
    if edition != package["edition"]:
        fail(f"MODULE.bazel builds edition {edition}, {CONFIG.relative_to(REPO)} says {package['edition']}")
    lock = load_lock()
    names = {lib.name for lib in libs}

    # Third-party dependencies: the union over the libraries.
    external = sorted({dep for lib in libs for dep in lib.external})
    forks = config.get("forks", {})
    if set(forks) != {c for c in patched if c in external}:
        fail(f"[forks] in {CONFIG.relative_to(REPO)} must list exactly the patched crates the libraries use: {sorted(c for c in patched if c in external)}")
    dependencies, fork_packages = {}, []
    for dep in external:
        if dep not in specs:
            fail(f"@{CRATES_REPO}//:{dep} has no crate.spec in MODULE.bazel")
        spec = specs[dep]
        version, attrs = resolved_crate(lock, dep)
        entry = {"version": version}
        if spec.get("default_features") is False:
            entry["default-features"] = False
        if spec.get("features"):
            entry["features"] = sorted(spec["features"])
        if dep in forks:
            fork_name, fork_version = build_fork(dep, attrs, version, forks[dep], package["name"], out)
            fork_packages.append(fork_name)
            entry = {"package": fork_name, "version": fork_version, "path": f"../{fork_name}", **{k: v for k, v in entry.items() if k != "version"}}
        dependencies[dep] = entry

    # Macros: #[macro_export] macros land at the merged root.
    merged = out / package["name"]
    shutil.rmtree(merged, ignore_errors=True)
    src_root = merged / "src"
    macros = {}
    for lib in libs:
        for path in sorted((REPO / lib.package / "src").rglob("*.rs")):
            for name in exported_macros(path.read_text()):
                if name in macros:
                    fail(f"#[macro_export] macro {name}! is defined in both {macros[name]} and {lib.name}")
                if name in names:
                    fail(f"#[macro_export] macro {name}! has the name of a crate")
                macros[name] = lib.name

    crate_attrs, stats = [], {"files": 0, "edits": 0}
    for lib in libs:
        lib_dir = REPO / lib.package
        dest = src_root / lib.name
        rewrite_names = (set(lib.deps) | lib.test_deps) - {lib.name}
        files = {p.relative_to(lib_dir / "src"): p.read_text() for p in sorted((lib_dir / "src").rglob("*.rs"))}
        for gen in lib.generated:
            target = dest / Path(gen.out).relative_to("src")
            generate_wgsl(gen, lib_dir, target, cpp_repo, out / "tools", edition)
            files[Path(gen.out).relative_to("src")] = target.read_text()
        if Path("mod.rs") in files or Path("lib.rs") not in files:
            fail(f"{lib.package}: expected src/lib.rs and no src/mod.rs")
        for rel, text in files.items():
            new, shadowed, edits = rewrite(text, lib.name, rewrite_names)
            stats["files"] += 1
            stats["edits"] += edits
            if shadowed:
                print(f"note: {lib.package}/src/{rel} binds {', '.join(sorted(shadowed))} itself; those paths stay local", file=sys.stderr)
            if rel == Path("lib.rs"):
                new, attrs = split_inner_attrs(new)
                crate_attrs += attrs
                rel = Path("mod.rs")
                own = sorted(name for name, owner in macros.items() if owner == lib.name)
                if own:
                    new = new.rstrip("\n") + (
                        "\n\n// Added by tools/release/assemble.py: #[macro_export] macros live at the\n"
                        f"// root of the merged crate; keep them reachable as {lib.name}::name!.\n"
                        + "".join(f"pub use crate::{name};\n" for name in own)
                    )
            write(dest / rel, new)

    lib_rs = [
        f"//! {package['description']}.",
        "//!",
        "//! Every library crate of https://github.com/louwers/maplibre-native-next is",
        "//! a public module of this crate, under its crate name. Generated by",
        "//! tools/release/assemble.py.",
        "",
        *merge_crate_attrs(crate_attrs),
        *(f"pub mod {lib.name};" for lib in libs),
    ]
    write(src_root / "lib.rs", "\n".join(lib_rs) + "\n")
    shutil.copy(REPO / "LICENSE.md", merged / "LICENSE.md")

    manifest = ["# Generated by tools/release/assemble.py from tools/release/package.toml,", "# the BUILD.bazel files, MODULE.bazel and MODULE.bazel.lock.", ""]
    manifest += toml_tables({"package": package})
    manifest += ["[lib]", 'path = "src/lib.rs"', "", "[dependencies]"]
    manifest += [f"{toml_key(k)} = {toml_value(v)}" for k, v in dependencies.items()]
    write(merged / "Cargo.toml", "\n".join(manifest) + "\n")

    members = fork_packages + [package["name"]]
    workspace = ["# Generated by tools/release/assemble.py.", "[workspace]", 'resolver = "3"', f"members = {toml_value(members)}"]
    write(out / "Cargo.toml", "\n".join(workspace) + "\n")
    write(out / "publish-order", "\n".join(members) + "\n")
    print(f"{out}: {len(libs)} modules, {stats['files']} files, {stats['edits']} path edits, {len(dependencies)} dependencies, {len(macros)} exported macros", file=sys.stderr)


def package_versions(out):
    order = (out / "publish-order").read_text().split()
    return [(name, tomllib.loads((out / name / "Cargo.toml").read_text())["package"]["version"]) for name in order]


def crate_files(data):
    files = {}
    with tarfile.open(fileobj=io.BytesIO(data)) as tar:
        for member in tar.getmembers():
            if member.isfile():
                rel = member.name.split("/", 1)[1]
                # Cargo.toml is normalized by cargo (differs between cargo
                # versions); Cargo.toml.orig is the manifest assemble wrote.
                if rel not in ("Cargo.toml", "Cargo.lock", ".cargo_vcs_info.json"):
                    files[rel] = tar.extractfile(member).read()
    return files


def status(args):
    out = Path(args.out).resolve()
    target = Path(args.target_dir).resolve() if args.target_dir else out / "target"
    missing = []
    for name, version in package_versions(out):
        local = target / "package" / f"{name}-{version}.crate"
        if not local.is_file():
            fail(f"{local} not found: run cargo package first")
        try:
            fetch(f"https://crates.io/api/v1/crates/{name}/{version}")
        except urllib.error.HTTPError as e:
            if e.code != 404:
                raise
            print(f"{name} {version}: not on crates.io", file=sys.stderr)
            missing.append(name)
            continue
        published = crate_files(fetch(f"https://static.crates.io/crates/{name}/{name}-{version}.crate"))
        ours = crate_files(local.read_bytes())
        differ = sorted(k for k in published.keys() | ours.keys() if published.get(k) != ours.get(k))
        if differ:
            fail(f"{name} {version} is on crates.io with different contents ({', '.join(differ[:5])}); bump the version in {CONFIG.relative_to(REPO)}")
        print(f"{name} {version}: already on crates.io, identical", file=sys.stderr)
    print(" ".join(missing))


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("assemble")
    p.add_argument("--cpp-repo", default=str(REPO.parent / "maplibre-native"))
    p.add_argument("--out", default=str(REPO / "target/release-crate"))
    p = sub.add_parser("status")
    p.add_argument("--out", default=str(REPO / "target/release-crate"))
    p.add_argument("--target-dir")
    sub.add_parser("version")
    args = parser.parse_args()
    if args.command == "assemble":
        assemble(args)
    elif args.command == "status":
        status(args)
    else:
        print(tomllib.loads(CONFIG.read_text())["package"]["version"])


if __name__ == "__main__":
    main()
