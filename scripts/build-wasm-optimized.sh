#!/bin/bash
# this_file: scripts/build-wasm-optimized.sh

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}[INFO]${NC} Building optimized WASM bundle..."

# Change to project root
cd "$(dirname "$0")"/..

# Install wasm-pack if not available
if ! command -v wasm-pack &>/dev/null; then
	echo -e "${YELLOW}[WARN]${NC} wasm-pack not found. Installing..."
	curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

# Install wasm-opt if not available
if ! command -v wasm-opt &>/dev/null; then
	echo -e "${YELLOW}[WARN]${NC} wasm-opt not found. Please install binaryen for optimal size reduction."
	echo "On macOS: brew install binaryen"
	echo "On Ubuntu: apt-get install binaryen"
fi

# Clean previous builds
echo -e "${GREEN}[INFO]${NC} Cleaning previous builds..."
rm -rf crates/wasm/pkg

# Build with size optimizations
echo -e "${GREEN}[INFO]${NC} Building WASM with aggressive size optimizations..."
cd crates/wasm

# Build with the wasm profile
RUSTFLAGS="-C opt-level=z -C lto=fat -C embed-bitcode=yes --cfg getrandom_backend=\"wasm_js\"" \
	wasm-pack build \
	--target web \
	--out-dir pkg \
	--release \
	--no-typescript \
	-- \
	--profile wasm \
	--features size-optimization

# Optimize with wasm-opt if available
if command -v wasm-opt &>/dev/null; then
	echo -e "${GREEN}[INFO]${NC} Running wasm-opt for additional size reduction..."

	# Get the original size
	ORIGINAL_SIZE=$(stat -f%z pkg/vexy-svgo_bg.wasm 2>/dev/null || stat -c%s pkg/vexy-svgo_bg.wasm)

	# Run wasm-opt with aggressive optimizations
	wasm-opt \
		-Oz \
		--enable-simd \
		--enable-bulk-memory \
		--enable-mutable-globals \
		--enable-nontrapping-float-to-int \
		--enable-sign-ext \
		--precompute-propagate \
		--dce \
		--vacuum \
		--strip-debug \
		--strip-producers \
		pkg/vexy-svgo_bg.wasm \
		-o pkg/vexy-svgo_bg_opt.wasm

	# Replace original with optimized version
	mv pkg/vexy-svgo_bg_opt.wasm pkg/vexy-svgo_bg.wasm

	# Get the optimized size
	OPTIMIZED_SIZE=$(stat -f%z pkg/vexy-svgo_bg.wasm 2>/dev/null || stat -c%s pkg/vexy-svgo_bg.wasm)

	# Calculate reduction
	REDUCTION=$((ORIGINAL_SIZE - OPTIMIZED_SIZE))
	PERCENTAGE=$((REDUCTION * 100 / ORIGINAL_SIZE))

	echo -e "${GREEN}[INFO]${NC} Size reduction: ${REDUCTION} bytes (${PERCENTAGE}%)"
	echo -e "${GREEN}[INFO]${NC} Original: ${ORIGINAL_SIZE} bytes"
	echo -e "${GREEN}[INFO]${NC} Optimized: ${OPTIMIZED_SIZE} bytes"
fi

# Generate minimal glue code
echo -e "${GREEN}[INFO]${NC} Generating minimal JavaScript glue code..."
cat >pkg/vexy-svgo_minimal.js <<'EOF'
// Minimal WASM loader for vexy-svgo
export async function initVexySvgo(wasmPath) {
    const response = await fetch(wasmPath);
    const bytes = await response.arrayBuffer();
    const module = await WebAssembly.instantiate(bytes, {
        wbindgen: {
            __wbindgen_throw: (ptr, len) => {
                throw new Error(getStringFromWasm(ptr, len));
            }
        }
    });
    
    const wasm = module.instance.exports;
    const memory = wasm.memory;
    
    // Helper to read strings from WASM memory
    function getStringFromWasm(ptr, len) {
        const buffer = new Uint8Array(memory.buffer, ptr, len);
        return new TextDecoder().decode(buffer);
    }
    
    // Helper to write strings to WASM memory
    function writeStringToWasm(str) {
        const encoder = new TextEncoder();
        const encoded = encoder.encode(str);
        const ptr = wasm.__wbindgen_malloc(encoded.length);
        new Uint8Array(memory.buffer, ptr, encoded.length).set(encoded);
        return [ptr, encoded.length];
    }
    
    return {
        optimize: (svg, config) => {
            const [svgPtr, svgLen] = writeStringToWasm(svg);
            const configStr = JSON.stringify(config || {});
            const [configPtr, configLen] = writeStringToWasm(configStr);
            
            const resultPtr = wasm.optimize(svgPtr, svgLen, configPtr, configLen);
            const result = getStringFromWasm(resultPtr, wasm.__wbindgen_get_result_len());
            wasm.__wbindgen_free(resultPtr);
            
            return JSON.parse(result);
        },
        
        version: () => {
            const ptr = wasm.get_version();
            const version = getStringFromWasm(ptr, wasm.__wbindgen_get_string_len());
            wasm.__wbindgen_free(ptr);
            return version;
        }
    };
}
EOF

# Create size report
echo -e "${GREEN}[INFO]${NC} Creating size report..."
cat >pkg/size-report.txt <<EOF
WASM Bundle Size Report
======================
Date: $(date)
Profile: wasm (opt-level=z, lto=fat)
Features: size-optimization, wee_alloc

File Sizes:
-----------
EOF

# Add file sizes to report
for file in pkg/*.{wasm,js}; do
	if [ -f "$file" ]; then
		SIZE=$(stat -f%z "$file" 2>/dev/null || stat -c%s "$file")
		printf "%-30s %10d bytes\n" "$(basename $file):" "$SIZE" >>pkg/size-report.txt
	fi
done

# Add gzipped sizes
echo -e "\nGzipped Sizes:" >>pkg/size-report.txt
echo "--------------" >>pkg/size-report.txt
for file in pkg/*.{wasm,js}; do
	if [ -f "$file" ]; then
		gzip -c "$file" >"$file.gz"
		SIZE=$(stat -f%z "$file.gz" 2>/dev/null || stat -c%s "$file.gz")
		printf "%-30s %10d bytes\n" "$(basename $file).gz:" "$SIZE" >>pkg/size-report.txt
		rm "$file.gz"
	fi
done

echo -e "${GREEN}[INFO]${NC} Size report saved to pkg/size-report.txt"
cat pkg/size-report.txt

echo -e "${GREEN}[SUCCESS]${NC} Optimized WASM build complete!"
echo -e "${GREEN}[INFO]${NC} Output files in: crates/wasm/pkg/"
