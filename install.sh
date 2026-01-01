#!/bin/bash
# Foodshare Tools Installer
# Builds and optionally installs the CLI tools

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Foodshare Tools Installer${NC}"
echo ""

# Check Rust
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Rust/Cargo not found${NC}"
    echo "Install from: https://rustup.rs"
    exit 1
fi

RUST_VERSION=$(rustc --version | cut -d' ' -f2)
echo -e "Rust version: ${GREEN}$RUST_VERSION${NC}"

# Build
echo ""
echo "Building release binaries..."
cargo build --release --workspace

echo ""
echo -e "${GREEN}Build complete!${NC}"
echo ""
echo "Binaries available at:"
echo "  - target/release/foodshare-ios"
echo "  - target/release/foodshare-android"
echo "  - target/release/lefthook-rs"

# Optional: Install to cargo bin
if [[ "$1" == "--install" ]]; then
    echo ""
    echo "Installing to ~/.cargo/bin..."
    cargo install --path bins/foodshare-ios
    cargo install --path bins/foodshare-android
    cargo install --path bins/lefthook-rs
    echo -e "${GREEN}Installed!${NC}"
fi

# Optional: Create symlinks for existing projects
if [[ "$1" == "--symlink" ]]; then
    echo ""
    echo "Creating symlinks for existing projects..."
    
    # Web project
    if [[ -d "../foodshare/tools/target/release" ]]; then
        ln -sf "$SCRIPT_DIR/target/release/lefthook-rs" "../foodshare/tools/target/release/lefthook-rs"
        echo "  ✓ foodshare/tools/target/release/lefthook-rs"
    fi
    
    # Android project
    if [[ -d "../foodshare-android/tools/target/release" ]]; then
        ln -sf "$SCRIPT_DIR/target/release/foodshare-android" "../foodshare-android/tools/target/release/foodshare-hooks"
        echo "  ✓ foodshare-android/tools/target/release/foodshare-hooks"
    fi
    
    # iOS project
    if [[ -d "../foodshare-ios/tools/target/release" ]]; then
        ln -sf "$SCRIPT_DIR/target/release/foodshare-ios" "../foodshare-ios/tools/target/release/foodshare-hooks"
        echo "  ✓ foodshare-ios/tools/target/release/foodshare-hooks"
    fi
    
    echo -e "${GREEN}Symlinks created!${NC}"
fi

# Optional: Clean build artifacts
if [[ "$1" == "--clean" ]]; then
    echo ""
    echo "Cleaning build artifacts..."
    cargo clean
    echo -e "${GREEN}Clean complete!${NC}"
    exit 0
fi

echo ""
echo "Usage:"
echo "  ./install.sh           # Build only"
echo "  ./install.sh --install # Build and install to ~/.cargo/bin"
echo "  ./install.sh --symlink # Build and create symlinks for existing projects"
echo "  ./install.sh --clean   # Clean all build artifacts"
