#!/usr/bin/env zsh

# Get the directory where this script is located
SCRIPT_DIR="${0:A:h}"

# Build and install the binary
echo "Building and installing arc..."
cargo install --path .

# Sign the binary
echo "Signing the binary..."
codesign -s - --force --identifier com.agilityrobotics.arc.backend --entitlements "$SCRIPT_DIR/entitlements.plist" ~/.cargo/bin/arc

echo "✅ Installation and signing complete!"

