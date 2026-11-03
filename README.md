# maplibre-native-next

A Rust port of the MapLibre Native core, tested against the C++ core in
`../maplibre-native`. Start with [PLAN.md](PLAN.md); working rules are in
[PORTING.md](PORTING.md).

```sh
scripts/bazel test //crates/... //vendor/...     # unit tests
scripts/bazel test //difftest/...                # Rust vs. C++
```
