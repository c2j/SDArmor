#!/bin/bash

# SD Armor Desktop Client Builder
# This script builds the desktop client for SD Armor

set -e  # Exit on any error

echo "🔨 Building SD Armor Desktop Client"

# Check if we're in the right directory
if [ ! -d "Desktop" ]; then
    echo "❌ Error: Desktop directory not found. Please run this script from the project root."
    exit 1
fi

# Build the desktop client
echo "🔧 Building desktop client..."
cd Desktop

# Check if Hyperscan feature should be enabled
if [ "$1" = "--with-hyperscan" ]; then
    echo "⚡ Building with Hyperscan support..."
    cargo build --release --features hyperscan_engine
else
    echo "⚙️  Building without Hyperscan support..."
    cargo build --release
fi

# Show build results
if [ -f "target/release/sdchat-scanner" ]; then
    echo "✅ Build successful!"
    echo "📦 Executable location: target/release/sdchat-scanner"
    echo "🚀 To run the application, execute:"
    echo "   ./target/release/sdchat-scanner"
else
    echo "❌ Build failed. Check the error messages above."
    exit 1
fi

cd ..