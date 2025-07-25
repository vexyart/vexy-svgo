#!/bin/bash
# Batch testing script for SVGO vs SVGN comparison
# Tests multiple files and generates comprehensive reports

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
COMPARE_SCRIPT="$SCRIPT_DIR/compare_outputs.sh"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

# Configuration
BATCH_RESULTS_DIR="$ROOT_DIR/test/results/batch_$(date '+%Y%m%d_%H%M%S')"
REPORT_FILE="$BATCH_RESULTS_DIR/batch_report.md"

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

info() {
    echo -e "${PURPLE}[INFO]${NC} $1"
}

usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Run batch comparison tests between SVGO and SVGN"
    echo ""
    echo "OPTIONS:"
    echo "  -h, --help          Show this help message"
    echo "  -d, --dir DIR       Test all SVG files in directory"
    echo "  -f, --files FILES   Test specific files (comma-separated)"
    echo "  -p, --plugin PLUGIN Test specific plugin only"
    echo "  -v, --verbose       Verbose output"
    echo "  --build             Build SVGN before running tests"
    echo "  --quick             Quick test (skip detailed analysis)"
    echo ""
    echo "EXAMPLES:"
    echo "  $0 --dir testdata/"
    echo "  $0 --files \"test1.svg,test2.svg\""
    echo "  $0 --plugin cleanupAttrs --dir testdata/samples/"
}

# Initialize counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
IDENTICAL_OUTPUTS=0
IMPROVED_OUTPUTS=0
DEGRADED_OUTPUTS=0

# Find SVG files
find_svg_files() {
    local dir="$1"
    find "$dir" -name "*.svg" -type f | head -50  # Limit to 50 files for testing
}

# Test a single file
test_file() {
    local file="$1"
    local plugin="$2"
    local verbose="$3"
    
    local basename=$(basename "$file" .svg)
    local test_dir="$BATCH_RESULTS_DIR/tests/$basename"
    mkdir -p "$test_dir"
    
    log "Testing: $file"
    
    # Prepare comparison command
    local cmd="$COMPARE_SCRIPT"
    if [[ -n "$plugin" ]]; then
        cmd="$cmd --plugin '$plugin'"
    fi
    if [[ "$verbose" == "true" ]]; then
        cmd="$cmd --verbose"
    fi
    cmd="$cmd --stats --diff '$file'"
    
    # Run comparison and capture output
    local output_file="$test_dir/comparison_output.txt"
    local exit_code=0
    
    if eval "$cmd" > "$output_file" 2>&1; then
        exit_code=0
        success "✓ $basename"
        ((PASSED_TESTS++))
        ((IDENTICAL_OUTPUTS++))
    else
        exit_code=$?
        if [[ $exit_code -eq 1 ]]; then
            warning "≠ $basename (outputs differ)"
            ((PASSED_TESTS++))
            
            # Check if SVGN output is smaller (improvement)
            if grep -q "SVGN output is.*bytes smaller" "$output_file"; then
                ((IMPROVED_OUTPUTS++))
                info "  → SVGN produced smaller output"
            else
                ((DEGRADED_OUTPUTS++))
                warning "  → SVGN produced larger output"
            fi
        else
            error "✗ $basename (test failed)"
            ((FAILED_TESTS++))
        fi
    fi
    
    ((TOTAL_TESTS++))
    
    # Save test metadata
    cat > "$test_dir/metadata.json" << EOF
{
    "file": "$file",
    "basename": "$basename",
    "plugin": "${plugin:-"default"}",
    "exit_code": $exit_code,
    "timestamp": "$(date -Iseconds)",
    "result": "$([ $exit_code -eq 0 ] && echo "identical" || [ $exit_code -eq 1 ] && echo "different" || echo "failed")"
}
EOF
    
    return $exit_code
}

# Generate markdown report
generate_report() {
    log "Generating batch report..."
    
    cat > "$REPORT_FILE" << EOF
# SVGO vs SVGN Batch Comparison Report

**Generated:** $(date)  
**Test Directory:** $BATCH_RESULTS_DIR

## Summary

| Metric | Count | Percentage |
|--------|--------|------------|
| Total Tests | $TOTAL_TESTS | 100% |
| Passed Tests | $PASSED_TESTS | $(echo "scale=1; $PASSED_TESTS * 100 / $TOTAL_TESTS" | bc -l)% |
| Failed Tests | $FAILED_TESTS | $(echo "scale=1; $FAILED_TESTS * 100 / $TOTAL_TESTS" | bc -l)% |
| Identical Outputs | $IDENTICAL_OUTPUTS | $(echo "scale=1; $IDENTICAL_OUTPUTS * 100 / $TOTAL_TESTS" | bc -l)% |
| VexySVGO Improvements | $IMPROVED_OUTPUTS | $(echo "scale=1; $IMPROVED_OUTPUTS * 100 / $TOTAL_TESTS" | bc -l)% |
| VexySVGO Degradations | $DEGRADED_OUTPUTS | $(echo "scale=1; $DEGRADED_OUTPUTS * 100 / $TOTAL_TESTS" | bc -l)% |

## Test Results

EOF

    # Add individual test results
    for test_dir in "$BATCH_RESULTS_DIR"/tests/*/; do
        if [[ -f "$test_dir/metadata.json" ]]; then
            local basename=$(basename "$test_dir")
            local metadata=$(cat "$test_dir/metadata.json")
            local result=$(echo "$metadata" | grep ' PROTECTED_1_ ' | cut -d'"' -f4)
            local file=$(echo "$metadata" | grep ' PROTECTED_1_ ' | cut -d'"' -f4)
            
            case "$result" in
                "identical")
                    echo "- ✅ **$basename** - Outputs identical" >> "$REPORT_FILE"
                    ;;
                "different")
                    echo "- ⚠️ **$basename** - Outputs differ" >> "$REPORT_FILE"
                    ;;
                "failed")
                    echo "- ❌ **$basename** - Test failed" >> "$REPORT_FILE"
                    ;;
            esac
            echo "  - File: \`$file\`" >> "$REPORT_FILE"
            
            if [[ -f "$test_dir/comparison_output.txt" ]]; then
                # Extract size information if available
                if grep -q "File sizes:" "$test_dir/comparison_output.txt"; then
                    local svgo_size=$(grep "SVGO:" "$test_dir/comparison_output.txt" | awk '{print $2}')
                    local svgn_size=$(grep "SVGN:" "$test_dir/comparison_output.txt" | awk '{print $2}')
                    echo "  - Sizes: SVGO=$svgo_size, SVGN=$svgn_size" >> "$REPORT_FILE"
                fi
            fi
            echo "" >> "$REPORT_FILE"
        fi
    done
    
    # Add detailed analysis section
    cat >> "$REPORT_FILE" << EOF

## Analysis

### Compatibility Status
$([ $IDENTICAL_OUTPUTS -eq $TOTAL_TESTS ] && echo "🎉 **Perfect Compatibility!** All outputs are identical." || echo "🔧 **Compatibility Issues Found** - $(echo "scale=1; ($TOTAL_TESTS - $IDENTICAL_OUTPUTS) * 100 / $TOTAL_TESTS" | bc -l)% of tests show differences.")

### Performance Summary
- **Improvements:** $IMPROVED_OUTPUTS tests show VexySVGO producing smaller output
- **Degradations:** $DEGRADED_OUTPUTS tests show VexySVGO producing larger output
- **No Change:** $IDENTICAL_OUTPUTS tests show identical output

### Recommendations
$(if [ $FAILED_TESTS -gt 0 ]; then echo "1. **Fix Failed Tests** - $FAILED_TESTS tests failed completely and need investigation"; fi)
$(if [ $DEGRADED_OUTPUTS -gt 0 ]; then echo "2. **Optimize Size Degradations** - $DEGRADED_OUTPUTS tests show larger output from SVGN"; fi)
$(if [ $IMPROVED_OUTPUTS -gt 0 ]; then echo "3. **Analyze Improvements** - $IMPROVED_OUTPUTS tests show SVGN improvements that could be ported to SVGO"; fi)

EOF

    success "Report generated: $REPORT_FILE"
}

# Main execution
main() {
    local test_dir=""
    local test_files=""
    local plugin=""
    local verbose=false
    local build_svgn=false
    local quick_test=false
    
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                usage
                exit 0
                ;;
            -d|--dir)
                test_dir="$2"
                shift 2
                ;;
            -f|--files)
                test_files="$2"
                shift 2
                ;;
            -p|--plugin)
                plugin="$2"
                shift 2
                ;;
            -v|--verbose)
                verbose=true
                shift
                ;;
            --build)
                build_svgn=true
                shift
                ;;
            --quick)
                quick_test=true
                shift
                ;;
            -*)
                error "Unknown option: $1"
                usage
                exit 1
                ;;
            *)
                error "Unexpected argument: $1"
                usage
                exit 1
                ;;
        esac
    done
    
    # Validate arguments
    if [[ -z "$test_dir" && -z "$test_files" ]]; then
        # Default to testdata directory
        test_dir="$ROOT_DIR/testdata"
        if [[ ! -d "$test_dir" ]]; then
            error "No test directory or files specified, and default testdata/ not found"
            usage
            exit 1
        fi
    fi
    
    # Create results directory
    mkdir -p "$BATCH_RESULTS_DIR/tests"
    
    log "Starting batch comparison tests..."
    log "Results directory: $BATCH_RESULTS_DIR"
    
    if [[ -n "$plugin" ]]; then
        log "Testing plugin: $plugin"
    fi
    
    # Build SVGN if requested
    if [[ "$build_svgn" == "true" ]]; then
        log "Building SVGN..."
        cd "$ROOT_DIR"
        cargo build --release
        cd - > /dev/null
    fi
    
    # Collect files to test
    local files_to_test=()
    
    if [[ -n "$test_dir" ]]; then
        log "Scanning directory: $test_dir"
        while IFS= read -r -d '' file; do
            files_to_test+=("$file")
        done < <(find_svg_files "$test_dir" -print0)
    fi
    
    if [[ -n "$test_files" ]]; then
        IFS=',' read -ra FILES <<< "$test_files"
        for file in "${FILES[@]}"; do
            if [[ -f "$file" ]]; then
                files_to_test+=("$file")
            else
                warning "File not found: $file"
            fi
        done
    fi
    
    if [[ ${#files_to_test[@]} -eq 0 ]]; then
        error "No SVG files found to test"
        exit 1
    fi
    
    log "Found ${#files_to_test[@]} files to test"
    
    # Run tests
    local start_time=$(date +%s)
    
    for file in "${files_to_test[@]}"; do
        test_file "$file" "$plugin" "$verbose"
        
        # Quick test mode - stop after 10 files
        if [[ "$quick_test" == "true" && $TOTAL_TESTS -ge 10 ]]; then
            warning "Quick test mode - stopping after 10 tests"
            break
        fi
    done
    
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    
    # Generate report
    generate_report
    
    # Final summary
    echo ""
    echo "==============================================="
    echo "           BATCH TEST SUMMARY"
    echo "==============================================="
    echo "Total Tests:       $TOTAL_TESTS"
    echo "Passed Tests:      $PASSED_TESTS"
    echo "Failed Tests:      $FAILED_TESTS"
    echo "Identical Outputs: $IDENTICAL_OUTPUTS"
    echo "SVGN Improvements: $IMPROVED_OUTPUTS"
    echo "SVGN Degradations: $DEGRADED_OUTPUTS"
    echo "Duration:          ${duration}s"
    echo "Results:           $BATCH_RESULTS_DIR"
    echo "Report:            $REPORT_FILE"
    echo "==============================================="
    
    if [[ $FAILED_TESTS -eq 0 ]]; then
        success "All tests completed successfully!"
        exit 0
    else
        error "$FAILED_TESTS tests failed"
        exit 1
    fi
}

# Run main function
main "$@"
