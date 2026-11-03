"""Machine-local tools for the wasm32 probe builds (wasm32_probe_build).

Not hermetic: paths into this NixOS machine's /nix/store. Clang compiles C for
wasm32 out of the box; the unwrapped one is used because the Nix wrapper adds
the host glibc headers. A real build would register a hermetic wasm32 C
toolchain instead (or drop the C dependencies on the web).
"""

PROBE_WASM32_CC = "/nix/store/c0jm2ghmasw8rljbixrhq3vw4hx2rgk8-clang-20.1.8/bin/clang"
PROBE_WASM32_AR = "/nix/store/c8f95avghs0b8m3vzvqc4fn9q058h7qp-llvm-21.1.8/bin/llvm-ar"
PROBE_WASM32_CFLAGS = "-resource-dir=/nix/store/diaqxdd3h63sribv98gdh6ml565yrwj3-clang-20.1.8-lib/lib/clang/20"
