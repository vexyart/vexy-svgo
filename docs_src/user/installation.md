---
nav_weight: 21
# this_file: docs/user/installation.md
layout: default
title: Installation
parent: User Guide
nav_order: 1
description: "Install Vexy SVGO on your system"
---

# Installation
{: .no_toc }

How to install Vexy SVGO on your system
{: .fs-6 .fw-300 }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Pre-built Binaries (Recommended)

The easiest way to get started is with our pre-built binaries, available for all major platforms.

### macOS

```bash
# Download and extract (Universal binary - works on Intel and Apple Silicon)
curl -sSL https://github.com/vexyart/vexy-svgo/releases/latest/download/vexy-svgo-macos.tar.gz | tar -xz

# Make executable and move to PATH
chmod +x vexy-svgo
sudo mv vexy-svgo /usr/local/bin/

# Verify installation
vexy-svgo --version
```

#### Alternative: Homebrew (Coming Soon)

```bash
# Will be available soon
brew install vexy-svgo
```

### Windows

```bash
# Download using PowerShell
Invoke-WebRequest -Uri "https://github.com/vexyart/vexy-svgo/releases/latest/download/vexy-svgo-windows.zip" -OutFile "vexy-svgo.zip"

# Extract
Expand-Archive -Path "vexy-svgo.zip" -DestinationPath "."

# Add to PATH or move to a directory in PATH
# Verify installation
.\vexy-svgo.exe --version
```

#### Alternative: Chocolatey (Coming Soon)

```bash
# Will be available soon
choco install vexy-svgo
```

### Linux

```bash
# Download and extract
curl -sSL https://github.com/vexyart/vexy-svgo/releases/latest/download/vexy-svgo-linux.tar.gz | tar -xz

# Make executable and move to PATH
chmod +x vexy-svgo
sudo mv vexy-svgo /usr/local/bin/

# Verify installation
vexy-svgo --version
```

---

## Build from Source

If you prefer to build from source or need the latest development version:

### Prerequisites

You'll need Rust and Cargo installed:

```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow the on-screen instructions and restart your shell
source ~/.cargo/env

# Verify installation
rustc --version
cargo --version
```

### Build Steps

```bash
# Clone the repository
git clone https://github.com/vexyart/vexy-svgo
cd vexy-svgo

# Build in release mode (optimized)
cargo build --release

# The binary will be at ./target/release/vexy-svgo
./target/release/vexy-svgo --version

# Optionally install system-wide
cargo install --path crates/cli
```

### Build Script

For a complete build including tests and verification:

```bash
# Use our build script
./build.sh all

# Or build specific components
./build.sh native     # CLI and library only
./build.sh wasm       # WebAssembly version
./build.sh docs       # Documentation
```

---

## Package Managers

### Rust/Cargo

Install as a Rust tool:

```bash
# Install from crates.io (when published)
cargo install vexy-svgo

# Or install from Git
cargo install --git https://github.com/vexyart/vexy-svgo vexy-svgo-cli
```

### npm (WebAssembly)

For Node.js and web development:

```bash
# Install WASM version via npm (when published)
npm install vexy-svgo-wasm

# Or install globally
npm install -g vexy-svgo-wasm
```

---

## Docker

Run Vexy SVGO in a container:

```bash
# Pull and run (coming soon)
docker run --rm -v $(pwd):/work vexyart/vexy-svgo input.svg -o output.svg

# Or build locally
git clone https://github.com/vexyart/vexy-svgo
cd vexy-svgo
docker build -t vexy-svgo .
docker run --rm -v $(pwd):/work vexy-svgo input.svg -o output.svg
```

---

## Verification

After installation, verify everything works:

```bash
# Check version
vexy-svgo --version

# Test basic functionality
echo '<svg xmlns="http://www.w3.org/2000/svg"><circle cx="50" cy="50" r="40"/></svg>' | vexy-svgo

# Run help to see all options
vexy-svgo --help
```

Expected output should show the optimized SVG and version information.

---

## Troubleshooting

### macOS: "Developer cannot be verified"

If you see a security warning on macOS:

```bash
# Remove quarantine attribute
xattr -dr com.apple.quarantine vexy-svgo

# Or go to System Preferences > Security & Privacy and allow the app
```

### Linux: Permission Denied

```bash
# Make sure the binary is executable
chmod +x vexy-svgo

# Check if the binary is in your PATH
echo $PATH
which vexy-svgo
```

### Windows: Antivirus Warning

Some antivirus software may flag the binary. This is a false positive common with Rust binaries. You can:

1. Add an exception for the file
2. Download from GitHub releases directly
3. Build from source

### Build Issues

If building from source fails:

```bash
# Update Rust toolchain
rustup update

# Clean and rebuild
cargo clean
cargo build --release

# Check system requirements
rustc --version  # Should be 1.74 or newer
```

---

## Next Steps

Once installed, continue to:

- [CLI Usage](/user/cli-usage/) - Learn command-line usage
- [Configuration](/user/configuration/) - Set up custom configurations
- [Interactive Demo](/demo/) - Try optimization in your browser

---

## System Requirements

- **Minimum**: Any 64-bit system (x86_64, ARM64)
- **Memory**: 50MB RAM for typical usage
- **Storage**: 10MB for the binary
- **OS**: macOS 10.12+, Windows 10+, Linux (glibc 2.17+)

For building from source:
- **Rust**: 1.74 or newer
- **Memory**: 1GB RAM for compilation
- **Storage**: 500MB for build dependencies