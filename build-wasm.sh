#!/bin/bash
# this_file: build-wasm.sh

set -e

echo "Building Vexy SVGO WebAssembly module..."

# Ensure wasm-pack is installed
if ! command -v wasm-pack &>/dev/null; then
	echo "Error: wasm-pack is not installed. Please install it first:"
	echo "  curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh"
	exit 1
fi

# Clean previous build
rm -rf pkg

# Function to build target with wasm-opt optimization
build_and_optimize() {
	local target=$1
	local out_dir=$2
	local features=$3
	local desc=$4

	echo "Building $desc..."
	RUSTFLAGS="-C opt-level=z -C lto=fat --cfg getrandom_backend=\"wasm_js\"" wasm-pack build crates/wasm --target "$target" --out-dir "../../$out_dir" --release -- --features "$features"

	# Optimize with wasm-opt if available
	if command -v wasm-opt &>/dev/null; then
		echo "  Optimizing with wasm-opt..."
		wasm-opt -Oz \
			--enable-simd \
			--enable-bulk-memory \
			--strip-debug \
			--strip-producers \
			--vacuum \
			--dce \
			"$out_dir"/vexy-svgo_bg.wasm \
			-o "$out_dir"/vexy-svgo_bg_opt.wasm
		mv "$out_dir"/vexy-svgo_bg_opt.wasm "$out_dir"/vexy-svgo_bg.wasm
		echo "  wasm-opt optimization complete"
	else
		echo "  wasm-opt not found, skipping additional optimizations"
	fi

	# Use wasm-snip to remove unused functions if available
	if command -v wasm-snip &>/dev/null; then
		echo "  Removing unused functions with wasm-snip..."
		wasm-snip --snip-rust-fmt-code \
			--snip-rust-panicking-code \
			"$out_dir"/vexy-svgo_bg.wasm \
			-o "$out_dir"/vexy-svgo_bg_snipped.wasm 2>/dev/null &&
			mv "$out_dir"/vexy-svgo_bg_snipped.wasm "$out_dir"/vexy-svgo_bg.wasm &&
			echo "  wasm-snip optimization complete" ||
			echo "  wasm-snip optimization skipped (no unused functions found)"
	else
		echo "  wasm-snip not found, skipping unused function removal"
	fi
}

# Build for different targets
build_and_optimize "web" "pkg-web" "wasm-default" "web target"
build_and_optimize "nodejs" "pkg-node" "wasm-default" "Node.js target"
build_and_optimize "bundler" "pkg-bundler" "wasm-default" "bundler target"
build_and_optimize "web" "pkg-minimal" "wasm-minimal" "minimal version"
build_and_optimize "web" "pkg-full" "wasm-full" "full version"

echo "WebAssembly builds complete!"
echo ""
echo "Generated packages:"
echo "  - pkg-web/      : For direct web browser usage"
echo "  - pkg-node/     : For Node.js applications"
echo "  - pkg-bundler/  : For webpack/rollup bundlers"
echo "  - pkg-minimal/  : Minimal build with critical plugins only"
echo "  - pkg-full/     : Full build with all plugins"
echo ""
echo "To use in a web project:"
echo "  import init, { optimize } from './pkg-web/vexy-svgo.js';"
echo "  await init();"
echo "  const result = optimize(svgString);"}
