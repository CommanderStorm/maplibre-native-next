# Releasing

The code is developed as Bazel crates (`crates/*`, `vendor/*`), but crates.io
gets a single crate, **maplibre-native-next**. In it, every library crate is a
public module under its own name: `maplibre_native_next::mln_map`,
`maplibre_native_next::mln_util`, `maplibre_native_next::earcut`, and so on.
There is one crate because crates.io rate-limits new crates and because
consumers such as maplibre-native-ffi want a single dependency.

HarfRust is patched in MODULE.bazel (`crates/mln_text/patches`), so it is
published as **maplibre-native-next-harfrust**: the harfrust 0.15.0 source from
crates.io with the patch applied. Its library is still called `harfrust`.

## Cutting a release

1. Set `version` in `tools/release/package.toml` and merge that change to
   main. If the harfrust patch or the harfrust version changed since the last
   release, also bump `[forks.harfrust] version`. crates.io never replaces a
   published version, and the workflow fails if a published version's
   contents differ from what it would publish.
2. Run the **release** workflow on main: Actions → release → Run workflow, or
   `gh workflow run release.yml --repo louwers/maplibre-native-next --ref main`.

If the workflow fails part way, fix the cause and run it again. Packages that
are already on crates.io with identical contents are skipped, and so is an
existing GitHub release.

## What the workflow does

`.github/workflows/release.yml`:

1. Checks out this repository. It needs no C++ checkout: the WGSL shaders are
   vendored in `crates/mln_webgpu/src/shaders/wgsl_sources.rs` (see below).
2. Installs stable Rust and runs `tools/release/assemble.py assemble`. This
   writes a Cargo workspace to `target/release-crate`:
   - `maplibre-native-next`: the `src/` of every `rust_library` under
     `crates/` and `vendor/`, each under `src/<crate name>/`. Its paths are
     rewritten for the new crate root: `crate::` becomes `crate::<name>::`,
     `$crate::` becomes `$crate::<name>::`, and `mln_util::…` becomes
     `crate::mln_util::…`. `#[macro_export]` macros end up at the crate root and
     are re-exported from their old crate module. The dependencies are the `@next_crates` deps of
     the libraries, with features from MODULE.bazel and minimum versions from
     MODULE.bazel.lock. Tests, tools, difftests and platform code are not
     included.
   - `maplibre-native-next-harfrust`: the crates.io source that
     MODULE.bazel.lock pins, checked against the lockfile's sha256 and patched
     with the `crate.annotation` patches.
3. Runs `cargo package` and `assemble.py status`, which compares each package
   with crates.io.
4. Runs `cargo publish --dry-run` on the missing packages. Packages published
   together resolve each other locally, so the merged crate verifies before
   the fork is on crates.io.
5. Runs `cargo publish` on the missing packages, the fork first.
6. Creates the tag `v<version>` on the workflow's commit and a GitHub release
   with the `.crate` files.

CI (`release-crate` in `ci.yml`) assembles the crate and runs `cargo check`
and `cargo package` on every push and PR. It catches drift early: a new
dependency, a rewrite the assembly cannot handle, or a manifest crates.io
would reject.

## Locally

```sh
python3 -I tools/release/assemble.py assemble
cd target/release-crate
cargo publish --dry-run --workspace
```

The assembly needs Python 3.11 or newer, GNU `patch` and network
access to crates.io. On NixOS, run the cargo commands inside
`nix develop path:.nix-shell#linux` if a build script cannot find a C
compiler or zlib.

## Secrets and settings

- **`CARGO_REGISTRY_TOKEN`** (repository secret): a crates.io API token of
  the account that will own the crates. It needs the `publish-new` and
  `publish-update` scopes, and can be limited to the crates
  `maplibre-native-next*`. crates.io tokens can expire, so renew it when it
  does. The account needs a verified email address to publish. After the
  first release, `cargo owner --add <user>` adds co-owners.
- **Workflow permissions**: the workflow asks for `contents: write` to push
  the tag and create the release. That works with the default repository
  setting (Settings → Actions → General). An organization or enterprise
  policy that caps `GITHUB_TOKEN` at read-only would block it.
- **Tag rules**: a ruleset or tag protection that covers `v*` must let GitHub
  Actions create tags.
- The workflow refuses to run on any branch other than the default branch.

## Shaders

The WGSL shaders come from the C++ core's `include/mln/shaders/webgpu/*.hpp`.
`scripts/vendor-shaders [maplibre-native checkout]` extracts them, unchanged,
into `crates/mln_webgpu/src/shaders/wgsl_sources.rs` (default checkout:
`../maplibre-native`). Rerun it when the C++ shaders change, and commit the
result.
