---
this_file: examples/c-ffi-usage/README.md
---

# C FFI Usage Example

This example shows how to link against `vexy-svgo-ffi` from C.

## Build the FFI library

From repository root:

```bash
cargo build --release -p vexy-svgo-ffi
```

This produces (platform-dependent):

- macOS: `target/release/libvexy_svgo_ffi.dylib`
- Linux: `target/release/libvexy_svgo_ffi.so`
- Windows: `target/release/vexy_svgo_ffi.dll`
- Static: `target/release/libvexy_svgo_ffi.a`

## Build the example (macOS/Linux)

```bash
cc -O2 examples/c-ffi-usage/main.c -o examples/c-ffi-usage/vexy-c-example \
  -L target/release -lvexy_svgo_ffi
```

If dynamic loading fails at runtime, set library path:

- macOS: `export DYLD_LIBRARY_PATH=target/release:$DYLD_LIBRARY_PATH`
- Linux: `export LD_LIBRARY_PATH=target/release:$LD_LIBRARY_PATH`

## Run

```bash
./examples/c-ffi-usage/vexy-c-example
```
