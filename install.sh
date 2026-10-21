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
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# The binaries this repository builds, and where each one lives. The
# list is the list the release workflow packages, so the two cannot drift
# apart silently.
BINARIES=(
  "fs-app:bins/fs-app"
  "fs-auto:bins/fs-auto"
  "fs-build:bins/fs-build"
  "fs-image:bins/fs-image"
  "fs-maestro:bins/fs-maestro"
  "fs-orchestrator:bins/fs-orchestrator"
  "fs-sim:bins/fs-sim"
  "fs-tui:bins/fs-tui"
  "fs-verify:bins/fs-verify"
  "fs-wasm:bins/fs-wasm"
  "fs-xccloud:bins/fs-xccloud"
  "foodshare-i18n:bins/foodshare-i18n"
  "foodshare-quadlet:crates/quadlet"
  "foodshare-swift:bins/foodshare-swift"
  "lefthook-rs:bins/lefthook-rs"
)

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
for entry in "${BINARIES[@]}"; do
    echo "  - target/release/${entry%%:*}"
done

# Optional: Install to cargo bin
if [ "$1" = "--install" ]; then
    echo ""
    echo -e "${BLUE}Installing binaries to Cargo bin...${NC}"
    for entry in "${BINARIES[@]}"; do
        cargo install --path "${entry#*:}"
    done
    echo -e "${GREEN}Installed!${NC}"
fi

# Optional: Create symlinks for the sibling repositories
#
# The hooks in the sibling repositories call these binaries by name. The
# paths below are the ones the repositories use.
if [[ "$1" == "--symlink" ]]; then
    echo ""
    echo "Creating symlinks for the sibling repositories..."

    for sibling in ../foodshare-app ../foodshare-web ../foodshare-backend; do
        if [[ -d "$sibling" ]]; then
            mkdir -p "$sibling/tools/target/release"
            for entry in "${BINARIES[@]}"; do
                ln -sf "$SCRIPT_DIR/target/release/${entry%%:*}" \
                    "$sibling/tools/target/release/${entry%%:*}"
            done
            echo "  ✓ $sibling/tools/target/release"
        fi
    done

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
echo "  ./install.sh --symlink # Build and create symlinks for the sibling repositories"
echo "  ./install.sh --clean   # Clean all build artifacts"
