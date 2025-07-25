#!/bin/bash
# SVGO vs SVGN output comparison tool
# Compares optimization outputs between SVGO and SVGN

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
TEMP_DIR="/tmp/svgn_comparison"
RESULTS_DIR="$ROOT_DIR/test/results"
SVGO_CMD="npx svgo"
SVGN_CMD="$ROOT_DIR/target/release/svgn"

# Create directories
mkdir -p "$TEMP_DIR"
mkdir -p "$RESULTS_DIR"

usage() {
    echo "Usage: $0 [OPTIONS] <input_file.svg>"
    echo ""
    echo "Compare SVGO and SVGN optimization outputs"
    echo ""
    echo "OPTIONS:"
    echo "  -h, --help          Show this help message"
    echo "  -v, --verbose       Verbose output"
    echo "  -p, --plugin PLUGIN Test specific plugin only"
    echo "  -d, --diff          Show detailed diff"
    echo "  -s, --stats         Show optimization statistics"
    echo "  --build             Build SVGN before running comparison"
    echo ""
    echo "EXAMPLES:"
    echo "  $0 testdata/samples/simple.svg"
    echo "  $0 --plugin cleanupAttrs testdata/samples/complex.svg"
    echo "  $0 --diff --stats testdata/samples/inline_styles/test_inline_simple.svg"
}

log() {
    echo -e "${BLUE}[$(date '+%H:%M:%S')]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Parse command line arguments
VERBOSE=false
PLUGIN=""
SHOW_DIFF=false
SHOW_STATS=false
BUILD_SVGN=false
INPUT_FILE=""

while [[ $# -gt 0 ]]; do
    case $1 in
    -h | --help)
        usage
        exit 0
        ;;
    -v | --verbose)
        VERBOSE=true
        shift
        ;;
    -p | --plugin)
        PLUGIN="$2"
        shift 2
        ;;
    -d | --diff)
        SHOW_DIFF=true
        shift
        ;;
    -s | --stats)
        SHOW_STATS=true
        shift
        ;;
    --build)
        BUILD_SVGN=true
        shift
        ;;
    -*)
        error "Unknown option: $1"
        usage
        exit 1
        ;;
    *)
        if [[ -z "$INPUT_FILE" ]]; then
            INPUT_FILE="$1"
        else
            error "Multiple input files specified"
            exit 1
        fi
        shift
        ;;
    esac
done

if [[ -z "$INPUT_FILE" ]]; then
    error "Input file required"
    usage
    exit 1
fi

if [[ ! -f "$INPUT_FILE" ]]; then
    error "Input file not found: $INPUT_FILE"
    exit 1
fi

# Build SVGN if requested
if [[ "$BUILD_SVGN" == "true" ]]; then
    log "Building SVGN..."
    cd "$ROOT_DIR"
    cargo build --release
    cd - >/dev/null
fi

# Check if required tools are available
check_tools() {
    log "Checking required tools..."

    if ! command -v npx &>/dev/null; then
        error "npx not found. Please install Node.js"
        exit 1
    fi

    if ! npx svgo --help &>/dev/null; then
        error "SVGO not found. Installing..."
        npm install -g svgo
    fi

    if [[ ! -f "$SVGN_CMD" ]]; then
        error "SVGN binary not found. Please build with: cargo build --release"
        exit 1
    fi

    success "All tools available"
}

# Run optimization with specific tool
run_optimization() {
    local tool="$1"
    local input="$2"
    local output="$3"
    local plugin="$4"

    if [[ "$tool" == "svgo" ]]; then
        local cmd="$SVGO_CMD"
        if [[ -n "$plugin" ]]; then
            cmd="$cmd --config='{\"plugins\": [{\"name\": \"$plugin\"}]}'"
        fi
        cmd="$cmd --input '$input' --output '$output'"
    else
        local cmd="$SVGN_CMD"
        if [[ -n "$plugin" ]]; then
            cmd="$cmd --enable $plugin --disable-all"
        fi
        cmd="$cmd '$input' > '$output'"
    fi

    if [[ "$VERBOSE" == "true" ]]; then
        log "Running: $cmd"
    fi

    eval "$cmd" 2>/dev/null || {
        error "Failed to run $tool optimization"
        return 1
    }
}

# Compare two files
compare_outputs() {
    local svgo_output="$1"
    local svgn_output="$2"

    # Basic comparison
    if cmp -s "$svgo_output" "$svgn_output"; then
        success "Outputs are identical! ✨"
        return 0
    fi

    warning "Outputs differ"

    # Size comparison
    local svgo_size=$(stat -f%z "$svgo_output" 2>/dev/null || stat -c%s "$svgo_output")
    local svgn_size=$(stat -f%z "$svgn_output" 2>/dev/null || stat -c%s "$svgn_output")

    echo "File sizes:"
    echo "  SVGO: $svgo_size bytes"
    echo "  SVGN: $svgn_size bytes"

    if [[ "$svgn_size" -lt "$svgo_size" ]]; then
        local savings=$((svgo_size - svgn_size))
        local percent=$(echo "scale=2; $savings * 100 / $svgo_size" | bc -l)
        success "SVGN output is ${savings} bytes smaller (${percent}% improvement)"
    elif [[ "$svgn_size" -gt "$svgo_size" ]]; then
        local increase=$((svgn_size - svgo_size))
        local percent=$(echo "scale=2; $increase * 100 / $svgo_size" | bc -l)
        warning "SVGN output is ${increase} bytes larger (${percent}% increase)"
    fi

    # Show diff if requested
    if [[ "$SHOW_DIFF" == "true" ]]; then
        echo ""
        echo "Detailed diff (SVGO vs SVGN):"
        echo "================================"
        diff -u "$svgo_output" "$svgn_output" || true
    fi

    return 1
}

# Calculate optimization statistics
show_statistics() {
    local original="$1"
    local svgo_output="$2"
    local svgn_output="$3"

    local original_size=$(stat -f%z "$original" 2>/dev/null || stat -c%s "$original")
    local svgo_size=$(stat -f%z "$svgo_output" 2>/dev/null || stat -c%s "$svgo_output")
    local svgn_size=$(stat -f%z "$svgn_output" 2>/dev/null || stat -c%s "$svgn_output")

    local svgo_savings=$((original_size - svgo_size))
    local svgn_savings=$((original_size - svgn_size))

    local svgo_percent=$(echo "scale=2; $svgo_savings * 100 / $original_size" | bc -l)
    local svgn_percent=$(echo "scale=2; $svgn_savings * 100 / $original_size" | bc -l)

    echo ""
    echo "Optimization Statistics:"
    echo "========================"
    echo "Original:     $original_size bytes"
    echo "SVGO output:  $svgo_size bytes (${svgo_savings} bytes saved, ${svgo_percent}%)"
    echo "SVGN output:  $svgn_size bytes (${svgn_savings} bytes saved, ${svgn_percent}%)"
}

# Main execution
main() {
    log "Starting SVGO vs SVGN comparison..."
    log "Input file: $INPUT_FILE"

    if [[ -n "$PLUGIN" ]]; then
        log "Testing plugin: $PLUGIN"
    fi

    check_tools

    # Generate output filenames
    local basename=$(basename "$INPUT_FILE" .svg)
    local svgo_output="$TEMP_DIR/${basename}_svgo.svg"
    local svgn_output="$TEMP_DIR/${basename}_svgn.svg"

    # Run optimizations
    log "Running SVGO optimization..."
    if ! run_optimization "svgo" "$INPUT_FILE" "$svgo_output" "$PLUGIN"; then
        exit 1
    fi

    log "Running SVGN optimization..."
    if ! run_optimization "svgn" "$INPUT_FILE" "$svgn_output" "$PLUGIN"; then
        exit 1
    fi

    # Compare outputs
    log "Comparing outputs..."
    if compare_outputs "$svgo_output" "$svgn_output"; then
        COMPARISON_RESULT=0
    else
        COMPARISON_RESULT=1
    fi

    # Show statistics if requested
    if [[ "$SHOW_STATS" == "true" ]]; then
        show_statistics "$INPUT_FILE" "$svgo_output" "$svgn_output"
    fi

    # Save results
    local result_dir="$RESULTS_DIR/$(date '+%Y%m%d_%H%M%S')_${basename}"
    mkdir -p "$result_dir"
    cp "$INPUT_FILE" "$result_dir/original.svg"
    cp "$svgo_output" "$result_dir/svgo.svg"
    cp "$svgn_output" "$result_dir/svgn.svg"

    # Generate report
    cat >"$result_dir/report.txt" <<EOF
SVGO vs VexySVGO Comparison Report
Generated: $(date)
Input: $INPUT_FILE
Plugin: ${PLUGIN:-"default preset"}

Outputs are identical: $([ $COMPARISON_RESULT -eq 0 ] && echo "YES" || echo "NO")

File sizes:
$(ls -la "$result_dir"/*.svg | awk '{print $9  PROTECTED_1_  $5  PROTECTED_2_ }')
EOF

    log "Results saved to: $result_dir"

    if [[ $COMPARISON_RESULT -eq 0 ]]; then
        success "Comparison completed successfully - outputs are identical!"
        exit 0
    else
        warning "Comparison completed - outputs differ"
        exit 1
    fi
}

# Cleanup on exit
cleanup() {
    if [[ -d "$TEMP_DIR" ]]; then
        rm -rf "$TEMP_DIR"
    fi
}

trap cleanup EXIT

# Run main function
main "$@"
