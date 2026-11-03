# Deviations from the C++ behaviour

Each entry: C++ location, what Rust does differently, why, and how tests
account for it.

- **`fmin`/`fmax` with signed zeros** (`util::min`/`util::max`, `util::clamp`).
  C++ calls the platform libm, whose choice between `-0.0` and `+0.0` is not
  specified. The port implements glibc's choice (return the first argument
  when the two compare equal). Difftests run on Linux/glibc.
- **Render-test outputs** (`render-test/runner.cpp`, `render_test.cpp`). C++
  writes `actual.png`/`diff.png` next to each test's style.json and the HTML
  report next to the manifest; the Rust runner writes them under `--output`
  (default `$TEST_UNDECLARED_OUTPUTS_DIR`) as `<test id>/actual.png` etc.,
  because the C++ tree is read-only and baselines are never overwritten.
  For the same reason `--update` is rejected. `--shuffle` reproduces the
  libstdc++ (Linux) order, not libc++'s.
- **Render-test comparison with a larger expected image**
  (`TestRunner::checkRenderTestResults`). C++ calls pixelmatch with the
  expected image's width and height over the actual image's buffer; if the
  expected image has more pixels, that reads and writes out of bounds. The
  Rust runner counts every pixel as different (difference 1) and tries the
  next `expected*.png`. Smaller or equal pixel counts behave as in C++.
- **Render-test runner robustness**. C++ terminates on exceptions it does not
  catch (`Map::setSize`, decoding a corrupt `expected.png`, an invalid
  `setCenter` LatLng, an unknown `setTileLodMode`); the Rust runner errors
  that test with the message and continues. A panic while creating or using
  the map errors the test and drops that map. `getValidPath` resolves
  against each manifest's directory (C++ caches the first one in a
  `static`; the runner parses one manifest per process). JSON parse errors
  carry serde_json's message instead of rapidjson's "... at offset N".
- **PNG codec** (`platform/default/src/mln/util/png_{reader,writer}.cpp`,
  `compression.cpp`). Decoding uses the `png` crate with libpng's transform
  chain (expand, strip 16, gray to RGB, alpha filler), so the pixels match;
  libpng's error texts after "failed to read invalid png: '" differ.
  Encoding writes the same chunks, but flate2 (miniz_oxide) compresses
  differently from zlib: same pixels, different bytes. That changes
  compressed `probeFileSize` metrics (they measure `util::compress`); no
  pixel comparison is affected. JPEG and WebP decoding are TODO(port).
- **[expression] `fmin`/`fmax` signed zeros, verified** (entry above):
  `//difftest/util:math_difftest` (`observed_signed_zero_behaviour` and the
  full ±0/NaN/inf cross product through `util::min`/`max`/`clamp`) shows the
  oracle returning the first argument on equal operands, at `-c fastbuild`
  and `--config=oracle-opt` (clang 20, glibc). `mln_math::{fmin, fmax,
  fminf, fmaxf}` implement exactly that.
- **[expression] x86-64 results for C++ undefined behaviour in
  CSSColorParser** (`vendor/csscolorparser/csscolorparser.cpp`,
  `clamp_css_byte`). `int64_t i = ::round(i)` overflows for values within 512
  of `INT64_MAX` (e.g. `rgb(9223372036854775807,0,0)` after `strtoll`
  saturates), and `uint8_t(NaN)` for `rgb(nan%,…)`. Both are UB; the port
  reproduces what clang emits on x86-64 (`cvttsd2si` gives `INT64_MIN`, so
  the channel clamps to 0, not 255; NaN gives 0). On arm64 the C++ saturates
  (255). Covered by `//difftest/util:csscolorparser_difftest`
  (`observed_quirks`, edge cases) on Linux x86-64.
- **[expression] C library number parsing is glibc's**
  (`csscolorparser::cstdlib::{strtof, strtod, strtoll}`, used by
  CSSColorParser and `mln_util::string::stof`). Correct rounding, glibc's
  ERANGE rules (overflow; underflow = tiny after rounding and inexact) and
  end pointers, "C" locale. `strtoll` is glibc ≥ 2.38's C23
  `__isoc23_strtoll` (what C++ gets with `_GNU_SOURCE`): base 0 and 2 accept
  `0b`. Other C libraries were not checked. Covered by
  `//difftest/util:cstdlib_difftest`.
- **[expression] `Padding::to_array`** returns `[f64; 4]` (seed API), C++
  `toArray()` returns `std::array<float, 4>`; same values. `to_float_array`
  has the C++ type.
- **[expression] JSON number parsing and object members**
  (`ConversionTraits<const JSValue*>`, src/mln/style/rapidjson_conversion.hpp).
  `Convertible` reads serde_json (with `float_roundtrip`, so numbers are
  correctly rounded like rapidjson's `kParseFullPrecisionFlag`, which the C++
  expression-test runner uses). Style JSON in C++ is parsed with
  `Parse<0>` (normal precision), which can be one ULP off for numbers with
  more than ~15 significant digits or large exponents; the integer literal
  `-0` is the unsigned 0 in rapidjson and `-0.0` in serde_json; with
  duplicate object keys rapidjson's `objectMember` finds the first, serde_json
  keeps the last. Not covered by tests yet.
- **[expression] Object iteration order** (`expression::Value`'s and
  `mapbox::base::Value`'s `std::unordered_map`, `Match` branches). The port
  uses `BTreeMap` (key order); C++ iterates in libstdc++ hash order. Affects
  the key order of `stringify`/`to-string`/`serialize` output for objects and
  the `each_child` order of `match` branches (C++ `Match::serialize` sorts,
  so serialization is unaffected there). The corpus runner compares objects
  key-wise (`deepEqual`), and `//difftest/expression` sorts keys in its
  canonical form.
- **[expression] Collator and `upcase`/`downcase`** (platform/default
  collator.cpp and string_stdlib.cpp, nunicode). Exact through the
  `//vendor/nunicode` port (see the nunicode entry for the invalid UTF-8 that
  nunicode writes for some 4-byte case mappings: the Rust strings hold U+FFFD
  there). `//difftest/expression` compares the expressions against the C++
  platform; the 7 `collator/*` corpus tests pass.
- **[expression] `number-format`** (platform/default `number_format.cpp`,
  ICU `NumberFormatter`). The port reimplements what ICU does for the root
  and English locales: double-conversion's shortest digits (ties to even),
  half-even rounding to max-fraction-digits, padding to min-fraction-digits,
  `,` grouping, the sign of negative numbers that round to zero, and for a
  currency ICU's currency precision with the en-US symbol of USD, EUR, JPY,
  GBP, CNY, INR, KRW, CAD, AUD (other codes: `CODE` + U+00A0). Other locales
  and currencies differ. `//difftest/expression` compares it against ICU for
  the root/en/en-US locales (generated numbers and digit counts, four
  currencies) and the corpus.
- **[expression] x86-64 results for C++ undefined behaviour in expressions**:
  `static_cast<int>`/`<int64_t>` of out-of-range doubles (value.cpp
  `writeJSON`, `slice`, `number-format` digit counts, `match` input
  rounding) reproduce `cvttsd2si` (INT_MIN / INT64_MIN). `Value::isSafeInteger
  (double)` of |x| >= 2^64 saturates (C++ UB, compiler dependent).
  `slice` with a negative normalized start index reads out of bounds in C++;
  the port starts at 0. `slice` with an erroring `fromIndex` dereferences the
  error in C++ (it checks the expression, not the result); the port returns
  the error. TODO(port) comments mark each.
- **[expression] Exceptions out of `evaluate`**: the
  `Interpolator<VariableAnchorOffsetCollection>` throws `std::runtime_error`
  for mismatched collections and `toVariableAnchorOffset` throws
  `bad_variant_access` for non-numeric offsets; both propagate out of
  `Expression::evaluate` in C++. The port returns an `EvaluationError` with
  the message for the former and panics for the latter.
- **[expression] Expression tests runner** (`expression-test/`). `UPDATE`
  mode and the HTML report are not ported; `-f` filters by substring, not
  regex; a panic in one test errors that test instead of aborting the run;
  fixture JSON errors carry serde_json's message. Legacy functions go
  through `mln_style`'s `convert_function_to_expression_for_type`.
- **[style] Layer/light observers** (style/layer.cpp, light.cpp,
  style_impl.cpp). C++ layers and the light hold a `LayerObserver*` /
  `LightObserver*` (the owning `Style::Impl`) and every setter calls
  `onLayerChanged` / `onLightChanged`. A Rust layer cannot point back into
  the collection that owns it, so `Style::Impl::get_layer_mut` /
  `get_light_mut` return guards that run `onLayerChanged` /
  `onLightChanged` on drop if the layer's or light's impl `Arc` was
  replaced (every notifying C++ setter replaces it). Several setter calls
  through one guard notify once; the final state is the same.
  `style::Observer` methods take `&self` (`Rc<dyn Observer>`), so an
  observer cannot read the style back from inside `on_update` (the C++
  TransitionOptionsPublishUpdates test does; the port reads it afterwards).
- **[style] JSON syntax errors** (style/parser.cpp, util/rapidjson.cpp
  `formatJSONParseError`). The style is parsed with serde_json, so the
  message after "Failed to parse style: " is serde_json's ("expected value
  at line 1 column 2"), not rapidjson's `GetParseError_En(code) + " at
  offset N"`. Valid styles parse the same. Duplicate object keys: rapidjson
  `operator[]`/`HasMember` find the first, serde_json keeps the last value
  (at the first key's position).
- **[style] Sprites and sources without storage** (style_impl.cpp
  `Style::Impl::parse`). Until FileSource/SpriteLoader are ported, the style
  acts as if it had a file source whose sprite requests fail: no "sprite"
  -> the C++ "non-existent sprite" path (loaded); with sprites ->
  `onSpriteRequested` + `onSpriteError` per sprite (C++ marks failed named
  sprites loaded), synchronously inside `load_json`. Source conversion
  returns "source type '<t>' is not ported yet" for every type (the parser
  logs it and skips the source, as for an invalid one), and the raster
  layer type is not registered (logged "Unsupported layer type! ...").
  render-tests: only background-opacity/overlay (M4) depends on this.
- **[style] Layer manager split** (layermanager/). `mln_style` holds the
  style half (`LayerFactory::createLayer`, `LayerManager::createLayer`); the
  C++ factories also create render layers, buckets and layouts, which the
  renderer does by `LayerTypeInfo` (mln_renderer is above mln_style).
- **[gfx] Crate cut around the drawable path** (renderer/layer_group.hpp,
  renderer/render_pass.hpp, renderer/paint_parameters.hpp). `LayerGroupBase`,
  `TileLayerGroup`, `LayerGroup` and the `mln::RenderPass` bitmask live in
  `mln_gfx::renderer` (the backends implement them); the layer groups keep no
  tweaker list (`LayerTweaker` needs style types, so `addLayerTweaker` /
  `runTweakers` live in mln_renderer); gfx/webgpu see `mln::PaintParameters`
  through the `mln_gfx::PaintParameters` trait; `render(RenderOrchestrator&,
  ...)` drops the unused orchestrator; `renderTileClippingMasks` takes tile
  ids instead of `RenderTiles`. `visitDrawables` has no try/catch (C++
  removes drawables whose callback throws and reports `onRenderError`).
  Behaviour on the render path is unchanged.
- **[gfx] Hash values** (util::hash users: `ProgramParameters::definesHash`,
  `ShaderGroup::propertyHash`/`getShaderName`, `ColorMode::hash`, the WebGPU
  pipeline and stencil hashes). The combination is boost's `hash_combine`,
  but the leaf hashes (std::hash of strings and ints) differ, and the defines
  map is ordered (C++: unordered_map order). These values are only cache
  keys and shader-group names ("BackgroundShader#<hex>"), never rendered.
- **[gfx] Shader groups store programs** (gfx/shader_group.hpp). C++ stores
  `shared_ptr<gfx::Shader>` and callers `static_pointer_cast` to
  `ShaderProgramBase`; the port stores `ShaderProgramBasePtr` (every
  registered shader is a program). Ported shader_registry gtests use stub
  programs that implement the trait.
- **[webgpu] wgpu instead of Dawn/wgpu-native** (src/mln/webgpu,
  headless_backend.cpp). Same calls through wgpu 30's Rust API, with these
  differences: the headless instance uses `WGPU_BACKEND` if set, else Vulkan
  on Linux (C++ wgpu-native: all backends, HighPerformance adapter) so tests
  run on lavapipe; WGSL compile and pipeline creation errors are caught with
  validation error scopes and give "no module"/"no pipeline" as the C++ null
  checks do, while any other validation error panics (wgpu's default
  handler; Dawn logs and invalidates the command buffer); the readback blocks
  with `Device::poll(Wait)` (C++: WaitAny / a 1000 x 1 ms poll loop); uniform
  buffers get their initial data by `queue.write_buffer` instead of
  `mappedAtCreation` (same contents at the next submit); a render pass
  without a stencil clear passes `stencil_ops: None` (C++: Undefined
  load/store ops); render passes are recorded with `forget_lifetime`, so
  wgpu rejects finishing an encoder with an open pass (C++ code never does).
- **[webgpu] Registered shader types** (renderer_backend.cpp `initShaders`).
  C++ registers all 31 WebGPU shader groups; the port registers those whose
  attribute/texture tables (src/mln/shaders/webgpu/*.cpp) are ported:
  BackgroundShader, BackgroundPatternShader, ClippingMaskProgram. The WGSL of
  all of them is extracted (tools/extract_wgsl.rs). Others arrive with their
  layers.
- **[expression] nunicode `b4_utf8` writes invalid UTF-8**
  (vendor/nunicode/include/libnu/utf8_internal.h). For 4-byte codepoints with
  bit 12 set, the C puts that bit into the third byte (mask `0x001F00`
  instead of `0x000F00`), so `platform::uppercase`/`lowercase` and the
  collator's `unaccent` emit invalid UTF-8 for mappings to such codepoints
  (e.g. Warang Citi U+118A0 <-> U+118C0). `//vendor/nunicode` reproduces the
  bytes (`uppercase_bytes`, `lowercase_bytes`, `unaccent_bytes`,
  `collator_compare` use them); the `String` functions (`uppercase`,
  `lowercase`, `unaccent`) decode them lossily (U+FFFD), because a Rust
  `String` cannot hold them. `//difftest/nunicode` checks the bytes exactly
  and the `String` against the lossy decoding of the C++ bytes.
- **[render] Layers without a source and the annotation source**
  (render_orchestrator.cpp `createRenderTree`). C++ handles background
  layers (and the background-as-clear-colour shortcut) while iterating over
  the style's *first source*; every C++ style has at least one, the
  annotation source that `AnnotationManager::onStyleLoaded` adds (plus its
  `org.maplibre.annotations.points` symbol layer). Annotations are not
  ported, so a Rust style may have no sources; the Rust loop then runs once
  without a source, handling only the source-less layers, exactly as the C++
  iteration over the annotation source does for them. When annotations are
  ported this becomes the C++ loop again. The output differs from C++ only
  in the absent annotation layer/source (empty without annotations).
- **[render] Style observer calls during `&mut Style`** (map_impl.cpp). C++
  `Map::Impl` handles style::Observer calls synchronously inside
  `Style::loadJSON`, layer mutations etc. In Rust the map hands out the
  style through a `StyleMut` guard; observer calls made while it is held are
  queued and replayed, in order, when it drops (before any other map call).
  Calls made at other times (sprite/source callbacks) are handled at once.
- **[render] LayerTweaker lists of layer groups** (layer_group.cpp
  `addLayerTweaker`/`runTweakers`). LayerTweaker is a renderer type and the
  layer groups live in mln_gfx, so the orchestrator keeps each group's weak
  tweaker list, filled through a change request that `processChanges`
  executes before the tweakers first run in the same frame (same order of
  effects as C++).
- **[storage] Actors stay on the thread that established them**
  (actor/mailbox.cpp, actor_ref.hpp, established_actor.hpp). A C++ message
  holds a raw reference to its target, which another thread may destroy
  (after `Mailbox::close` waits for a running `receive`). Safe Rust cannot
  send that reference, so the target lives in a registry of its thread
  (`mln_actor::LocalObject`) and messages are closures over `&O`; `close()`
  and `receive()` run on that thread, and the single mailbox mutex gives the
  C++ guarantees (no message after `close`, pushes to a closed mailbox are
  dropped). Actors scheduled on a thread pool (`Actor<O>` with
  `Scheduler::GetBackground()`) are a TODO(port); file sources, their
  requests and `util::Thread` actors behave as in C++. `schedule_and_reply_value`
  keeps the reply closure on the calling thread for the same reason.
- **[storage] `FileSourceManager::getFileSource` runs the factory without
  its lock** (C++: a recursive mutex held across the factory, which calls
  back into the manager for MainResourceLoader). On one thread the result
  is the same; if two threads create the same (type, options) file source at
  once, the first one registered is returned to both.
- **[storage] DatabaseFileSource's tile server options**
  (database_file_source.cpp `DatabaseFileSourceThread`). C++ reads them from
  the online file source, which is not ported (FileSourceManager has no
  Network factory yet, as for Asset, FileSystem, Mbtiles and Pmtiles: those
  file sources are `None` in MainResourceLoader). The port takes them from
  the same ResourceOptions the online file source would be created with.
- **[storage] OfflineDatabase write path not ported** (offline_database.cpp
  `put*`, eviction, regions, pack). In read-write mode `put` returns
  `(false, 0)` and `DatabaseFileSource::forward` stores nothing; opening,
  schema creation, migrations, error recovery, `get`/`getResource`/`getTile`
  (incl. the `accessed` update in read-write mode) and read-only reopening
  are ported. `disabled()` with a zero ambient cache size counts the regions
  instead of `listRegions()` (same answer unless a region definition fails
  to parse). `Query::get<std::string>` of a REAL column formats with Rust's
  `{}` instead of SQLite's `%!.15g` (no such columns in the schema's text or
  blob fields). `util::decompress` error texts are flate2's. Covered by
  `//difftest/storage` on every row of both cache databases.
- **[sprite] Sprite JSON parsing** (sprite_parser.cpp `parseSprite`, rapidjson
  `Parse<0>`). serde_json instead of rapidjson: the message after "Failed to
  parse JSON: " is serde_json's (C++: "Invalid value. at offset 0" etc.); with
  duplicate keys rapidjson visits every member and serde_json only the last
  value; invalid UTF-8 is a parse error (rapidjson does not validate); the
  integer literal `-0` is a double in serde_json (so `"x": -0` warns, C++ reads
  0). Everything else, incl. warnings, errors and pixels, is covered by
  `//difftest/storage` (fixtures, cache-database sprites, generated JSON).
- **[style] Sprites through the SpriteLoader** (supersedes the style entry
  "Sprites and sources without storage" for sprites). `Style::new(px)` is the
  C++ style with a null file source: a parsed style reports the sprite error
  "Unable to find resource provider for sprite url." and is not loaded, as in
  C++. `Style::with_file_source(..)` loads sprites through the SpriteLoader.
  `loadURL` with a file source is a TODO(port) (it reports a
  StyleLoadException "TODO(port): ..."); C++ requests and parses the URL.
- **[map] Map thread pool** (map_impl.cpp). C++ passes
  `frontend.getThreadPool()` (the renderer backend's tagged background pool)
  to the style; `RendererFrontend` has no thread pool yet, so Map::Impl uses
  the background pool with a fresh tag. Only the grouping for
  `waitForEmpty` differs.
- **[storage] StubFileSource timer** (test/src/mln/test/stub_file_source.cpp,
  tests only). The asynchronous stub polls from an AsyncTask re-armed every
  millisecond while requests are pending, not a 1 ms `util::Timer`
  (RunLoop timers are a TODO(port)).
- **[codegen] Style code generator** (scripts/generate-style-code.mjs,
  style-code.mjs, style-spec.mjs, the .ejs templates; ported as
  //tools/style-codegen). Same decisions, with these implementation
  differences: (1) properties are sorted by bytes instead of
  `Intl.Collator("en-US")`; for names of lowercase ASCII letters, digits and
  '-' ICU's root order is the byte order, and the generator refuses any other
  name; (2) default colors are parsed with the vendored csscolorparser port
  instead of the npm one, which agree for alpha 0 and 1, the only alphas of
  the spec defaults; the generator refuses fractional alphas. Checked against
  the C++ generated headers: every tag's base class (including the
  attribute/uniform names), default-value initializer, `name()`, the property
  sets' order, kPaintPropertyCount, the Property enum, the property tables,
  the setPropertyInternal conversion groups and the LayerTypeInfo values
  agree for all 11 layer types; //difftest/style_codegen checks the values.
- **[codegen] Generated property tags** (src/mln/style/layers/*_layer_properties.hpp).
  The data-driven tags don't carry the `A`/`U` template arguments
  (`attributes::radius, uniforms::radius`); the C++ base class with them is
  each tag's doc comment, for the renderer's drawables to look up.
  `Unevaluated::stringify` / `Layer::Impl::stringifyLayout` are not ported
  (TODO(port); nothing in the C++ core calls stringifyLayout).
  `PossiblyEvaluated::evaluate<P>(z, feature, ...)` is `evaluate`,
  `evaluate_canonical`, `evaluate_state`, `evaluate_images`,
  `evaluate_images_canonical`; the argument combinations C++ has no overload
  for (feature state or images with a non-data-driven property, any
  cross-faded property) are `unreachable!` instead of compile errors.
- **[codegen] FormatSectionOverrides** (symbol_layer_impl.hpp). The style
  half (`hasOverrides`, `hasOverride`, `hasPaintPropertyDifference`) is in
  `layers::symbol_layer_impl::SymbolLayerPaintPropertyOverrides`, written out
  for the one overridable property (text-color) instead of folded over a
  type list; `setOverride(s)` / `updateOverride(s)` (render side,
  `FormatSectionOverride`) are TODO(port) with RenderSymbolLayer (M5), so the
  ported SymbolLayerOverrides gtest checks the style half only.
- **[codegen] //difftest/style_codegen inputs**. For `resolvedImage`
  properties the sample set has no constant expression such as
  `["case", true, "a", "b"]`: the `image` coercion is not constant-folded,
  and an expression that is feature-, zoom- and runtime-constant but not a
  literal hits `assert(false)` in conversion/property_value.cpp (a C++
  precondition).
- **[tile] Strings from tiles are UTF-8** (vendor/vector-tile, protozero
  `get_string`, `VectorMVTTileLayer::getName`). C++ keeps layer names, keys
  and string values as `std::string` bytes; `mln::Value::String` and the
  layer name are Rust `String`s, so invalid UTF-8 becomes U+FFFD
  (`from_utf8_lossy`). Layer lookup, layer order and the key comparison of
  `getValue` work on the raw bytes as in C++, so only these differ: string
  values and property keys with invalid UTF-8 (lossy), two keys that are
  equal only after the lossy conversion (C++ keeps both, the port the first),
  and a layer whose name is not UTF-8 cannot be requested through `&str`
  (style `source-layer`s are JSON strings, so never in practice).
  `//difftest/tile` normalizes the C++ strings UTF-8-lossy before comparing;
  test/fixtures/map/invalid_utf8 matches after normalization.
- **[tile] protozero wire-type assertions** (`protozero_assert(has_wire_type(..))`
  in pbf_reader.hpp `get_*()`). `vector_tile.hpp` never checks wire types,
  so a known field with the wrong wire type aborts a debug (non-NDEBUG) C++
  build; release C++ decodes it as if the type matched. The port always
  does what release C++ does. The data-independent preconditions ("call
  next() before accessing field value") are `debug_assert!`s, and debug
  builds reset `tag()` to 0 after a get that skips bytes, as C++ does.
  `//difftest/tile` compares inputs that reach such an assertion (11 fixture
  files: glyph PBFs and non-MVT blobs; and all byte-mutated tiles) only with
  `--config=oracle-opt`.
- **[tile] Exceptions out of the GeometryTile\* trait methods**
  (vector_mvt_tile_data.cpp). C++ `getLayer`, `getFeature`, `getValue`,
  `getProperties` and `getGeometries` throw on malformed tiles (protozero
  exceptions, `std::out_of_range`, `std::runtime_error`); the
  `mln_tile_id::geometry_tile_data` traits have no error channel. The
  `VectorMVTTile*` types have `try_*` methods returning
  `Result<_, vector_tile::Error>` (same exception type and `what()`), and
  the trait methods panic with the `what()` text where the C++ exception
  would escape. The C++ GeometryTileWorker catches everything around
  parsing (`catch (...)` -> `onError`); its port must use `catch_unwind` or
  the `try_*` methods there.
- **[tile] x86-64 results of undefined casts and overflows on the tile
  path**: `getGeometries` with extent 0 (scale = inf) converts NaN with
  `static_cast<int16_t>`, which clang compiles to cvttss2si (0x80000000)
  keeping the low 16 bits, i.e. 0, which Rust's `as` also gives;
  `convertGeometry` (lng/lat to tile coordinates) casts out-of-range and
  NaN doubles to int16 before its clamp, reproduced as cvttsd2si + low 16
  bits; `signedArea` and wagyu's int32 sums/products wrap. Covered by
  `//difftest/tile` (extent 0, NaN/inf/huge lng/lat, ±32768 coordinates).
- **[tile] libstdc++ algorithm order** (tile/geometry_tile_data.cpp
  `limitHoles`, vendor/wagyu). The order `std::nth_element` leaves the
  kept holes in, the result of wagyu's `std::stable_sort` with
  `intersect_list_sorter` (a ULP-tolerant comparator, not a strict weak
  ordering) and the order of equal keys in wagyu's `std::unordered_multimap`
  are libstdc++'s; the port reproduces those implementations
  (`mln_tile::stl_algo::nth_element`, `//vendor/libstdcxx`'s
  `stable_sort_by`, wagyu's `ConnectionMap`). C++ built against libc++ or
  MSVC's library can give a
  different hole order or clipper output. Checked against the Linux oracle.
- **[text] HBShaper on HarfRust instead of HarfBuzz 8.2.0 (HB_TINY) + hb-ft**
  (src/mln/text/harfbuzz_impl.cpp). The C++ core shapes with its vendored
  HarfBuzz 8.2.0 built with `HB_TINY`, on an hb-ft font (FreeType glyph
  mapping and advances, scale 24 px in 26.6). The port shapes with HarfRust
  0.15 (HarfBuzz 15.0.0 behaviour) and reproduces what it can of that setup:
  hb-ft's nominal/variation glyph and advance callbacks on a port of
  FreeType 2.13's charmaps, sizing and `FT_Get_Advance`
  (`crates/mln_text/src/{ft,hb_ft.rs}`; `//difftest/text:ft_difftest`:
  19.1M charmap lookups and 36k advances identical on 145 fonts), hb-ft's
  scale, HarfBuzz 8.2.0's own Unicode 15.0 tables for the script
  segmentation (`hb_ucd_table.rs`, generated; every codepoint identical,
  `:ucd_difftest`), and HB_TINY's compiled-out features by hiding the
  tables HarfBuzz never reads (morx, mort, kerx, kern, ankr, trak, feat;
  HB_NO_AAT, HB_NO_OT_KERN), dropping GSUB/GPOS FeatureVariations
  (HB_NO_VAR) and passing `frac=0 numr=0 dnom=0` (HB_NO_OT_SHAPE_FRACTIONS).
  `//difftest/text:shaping_difftest` shapes 407 strings (107 hand-written,
  300 generated) with 145 fonts (HarfBuzz's perf/API test fonts and the
  render tests' font-faces fonts): 55587 of 59015 cases are bit-identical
  (55446 before the HarfRust patch below). Every one of the 3428 others is
  minimized to a reproducer and has one of causes 1-3 and 6 (counts at
  fastbuild and `--config=oracle-opt` alike):
  1. *Unicode data* (1691 cases, 22 minimal texts): the text has a
     codepoint unassigned in Unicode 15.0 (HB_NO_UCD_UNASSIGNED gives those
     its neighbours' properties) or whose script or combining class differs
     between HarfBuzz 8.2.0 (Unicode 15.0) and HarfRust (Unicode 18.0):
     29210 codepoints differ in script, 80 in combining class, nearly all
     unassigned in 15.0. Reproducer: U+05CF alone (any font without it):
     C++ advance 0 (treated as a mark), Rust the .notdef advance.
  2. *Fallback mark positioning* (1649 cases, 166 minimal texts):
     HB_NO_OT_SHAPE_FALLBACK turns it off in C++; HarfRust always runs it
     for fonts without (usable) GPOS — Hebrew needs a `hebr` GPOS script —
     and, with no glyph extents (hb-ft's extents are unused by HB_TINY, so
     the port returns none), it zeroes the advance of every non-spacing
     mark after a base, also marks GDEF does not class as marks (typically
     .notdef). Reproducer: Noto Sans devanagari.ttf, "ו\u{5b9}": C++
     advances 14.40625 14.40625, Rust 0 14.40625 (visual order).
  3. *Vowel constraints* (79 cases, 1 text): HB_NO_OT_SHAPER_VOWEL_CONSTRAINTS
     in C++; HarfRust inserts U+25CC. Reproducer: "अॅ" (U+0905 U+0945),
     every font: 2 glyphs in C++, 3 in Rust.
  4. *HarfRust bug, rounding* (27 cases, 12 texts; fixed by the patch):
     HarfBuzz (8.2.0 and
     15.0.0) rounds scaled anchor positions with `roundf` = `floor(x + .5)`;
     HarfRust's `Scale::scale_x_f` uses `f32::round`, so -n.5 ends one unit
     lower. Reproducer: perf/fonts/Roboto-Regular.ttf "Ҙ̥" (U+0498 U+0325):
     the mark's anchor delta is -10 units * 0.75 = -7.5: HarfBuzz -7,
     HarfRust -8 (HBShapeAdjust y_offset 0.109375 vs 0.125).
  5. *HarfRust bug, fallback spaces* (114 cases, 5 texts; fixed by the
     patch): HarfBuzz sizes
     U+2000..U+200A etc. from its scale, `(x_scale + n/2) / n`; HarfRust
     scales `(upem + n/2) / n`. Reproducer: Noto Sans devanagari.ttf (upem
     1000) U+2004: C++ 512/64 = 8.0, Rust 511/64.
  6. *FreeType's synthesized charmap* (9 cases, 1 font): RanaKufi-Regular.subset.otf
     has no Unicode cmap; FreeType's CFF driver synthesizes one from the
     glyph names (psnames + the Adobe Glyph List), which is not ported
     (TODO(port) in ft/face.rs), so its characters map to .notdef.
  `crates/mln_text/patches/harfrust_hb_rounding.patch` fixes 4 and 5 in
  HarfRust; MODULE.bazel applies it (`crate.annotation`), to be
  upstreamed. With it both counts are 0 and nothing else changes; the
  difftest no longer accepts them. 1-3 need HarfRust features (HB_TINY-like
  switches or Unicode tables) that do not exist.
- **[text] Glyph rasterization** (FreeTypeFace::rasterizeGlyph,
  freetype.cpp). The port loads hinted outlines with skrifa
  (`HintOptions::auto_fallback`, grayscale: FreeType's FT_LOAD_DEFAULT engine
  choice) and renders them with a port of FreeType's smooth rasterizer
  (`ft/grays.rs`: ftgrays.c's FT_INT64 paths and ftsmooth.c's overlap
  oversampling) into `ft_glyphslot_preset_bitmap`'s box. Differences come
  from skrifa's hinting engines vs FreeType 2.13's. Measured by
  `//difftest/text:rasterize_difftest` on 8814 glyphs (up to ~400 per
  font, 145 fonts): metrics identical for 8189 (92.9%), bitmaps
  identical for 5991 (68.0%), bitmap size differs for 414 (4.7%); in
  same-size bitmaps 41237 of 3757221 pixels (1.1%) differ, by 18.35 on
  average, at most 255. Per engine, fonts with no differing pixel: TrueType
  bytecode 37 of 58, PostScript 49 of 51, autohinter 20 of 29. The SDF step
  (`util::transformRasterToSDF`, ported as `mln_util::tiny_sdf`) is
  bit-identical on all 8814 C++ bitmaps. Embedded-bitmap glyphs (CBDT/
  sbix/EBDT without outlines) are TODO(port) (empty glyph).
- **[geometry] x86-64 results for out-of-range `double` -> integer casts**
  (geojson-vt `InternalTile::transform` `static_cast<int16_t>(round(..))`,
  supercluster `getTile` (`point<int16_t>` from doubles, `uint32_t z2 =
  pow(2, z)`), geojson-vt `splitTile` / kdbush `select` (`uint32_t`/`size_t`
  from double), earcut `zOrder`). Out-of-range values are UB in C++; the
  ports reproduce clang's x86-64 code (`cvttsd2si` into a 32-bit register
  for `int16_t`/`int32_t`, keeping the low bits; 64-bit for `uint32_t`;
  `//vendor/libstdcxx` `cvt`), not Rust's saturating `as`. Reachable with
  unclipped `geoJSONToTile` (custom geometry sources with `clip: false`) at
  high zooms; `//difftest/geometry:geojson_vt_difftest` covers it on Linux
  x86-64. arm64 C++ saturates instead.
- **[geometry] mapbox::geojson on serde_json** (geojson.hpp
  `geojson_impl.hpp`, `//vendor/geojson`). `convert` takes a
  `serde_json::Value` (C++: a rapidjson value), so the JSON-level
  differences of the expression entry above apply: the last of duplicate
  keys wins (C++ `FindMember`/`emplace`: the first), the literal `-0` is the
  double -0.0 (C++: unsigned 0), `parse` rounds every number correctly (C++
  `parse` uses rapidjson's default precision, which can be one ULP off for
  long decimals) and its syntax errors carry serde_json's text (C++:
  "<offset> - <rapidjson message>"). Where the C++ calls a rapidjson accessor
  on the wrong type (a non-number coordinate, a non-array line string in a
  MultiLineString, a non-string "type" in an error message), rapidjson
  asserts in debug builds and reads garbage in release builds; the port
  returns an error ("coordinates must be numbers", the container error,
  "" as the type name). `to_json` (C++ `convert(geojson, allocator)`) writes
  NaN/infinite doubles as `null` (a rapidjson value can hold them). Property
  maps are key-ordered (`BTreeMap`), so `stringify`/`to_json` write members
  in key order where C++ writes `std::unordered_map` order; values and every
  other byte are the same (`//difftest/geometry:geojson_difftest` compares
  byte-exact when no property object has more than one key, structurally
  otherwise).
- **[geometry] Handles instead of pointers** (shelf-pack `Bin*`, earcut
  `Node*`). `ShelfPack` returns `BinRef`s (shelf and bin index; equal iff
  the C++ pointers are equal) read through `ShelfPack::bin`; `stats()` and
  `shelves()` expose two private members for tests. Same packing, reuse and
  refcounts (`//difftest/geometry:shelf_pack_difftest`).
- **[geometry] std::stable_sort without a temporary buffer**
  (`//vendor/libstdcxx`). libstdc++ falls back to `__inplace_stable_sort` /
  `__stable_sort_adaptive_resize` when it cannot allocate the `(n + 1) / 2`
  buffer; the port only has the full-buffer path (an allocation failure
  aborts). Results are identical whenever the C++ allocation succeeds.
