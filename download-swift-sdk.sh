#!/bin/bash
# Download Swift 6.3 nightly toolchain + Android SDK with curl resume support
set -e

SNAPSHOT="swift-6.3-DEVELOPMENT-SNAPSHOT-2026-02-14-a"
TOOLCHAIN_URL="https://download.swift.org/swift-6.3-branch/xcode/${SNAPSHOT}/${SNAPSHOT}-osx.pkg"
TOOLCHAIN_PKG="/tmp/${SNAPSHOT}-osx.pkg"

echo "=== Step 1: Download Swift 6.3 nightly toolchain (1.78 GB) ==="
echo "URL: ${TOOLCHAIN_URL}"
echo ""

MAX_RETRIES=20
RETRY=0
while [ $RETRY -lt $MAX_RETRIES ]; do
    RETRY=$((RETRY + 1))
    echo "--- Attempt ${RETRY}/${MAX_RETRIES} ---"
    if curl -L -C - -o "${TOOLCHAIN_PKG}" --connect-timeout 30 --max-time 300 --retry 3 "${TOOLCHAIN_URL}"; then
        echo "Download complete!"
        break
    else
        echo "Download interrupted, resuming in 3s..."
        sleep 3
    fi
done

if [ ! -f "${TOOLCHAIN_PKG}" ]; then
    echo "ERROR: Failed to download toolchain after ${MAX_RETRIES} attempts"
    exit 1
fi

FILE_SIZE=$(stat -f%z "${TOOLCHAIN_PKG}" 2>/dev/null || echo 0)
echo "Downloaded file size: ${FILE_SIZE} bytes (~$((FILE_SIZE / 1024 / 1024)) MB)"

echo ""
echo "=== Step 2: Install toolchain via swiftly ==="
/opt/homebrew/bin/swiftly install --assume-yes "${SNAPSHOT}" --post-install-file="${TOOLCHAIN_PKG}" 2>&1 || {
    echo "swiftly install failed, trying pkg installer..."
    sudo installer -pkg "${TOOLCHAIN_PKG}" -target /
}

echo ""
echo "=== Step 3: Re-run skip android sdk install ==="
export PATH="/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin:$PATH"
skip android sdk install --version nightly-6.3

echo ""
echo "=== Done ==="
