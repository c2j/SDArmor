#!/bin/bash

# SD Armor Development Environment Setup Script
# This script sets up the development environment for SD Armor

set -e  # Exit on any error

echo "🚀 Setting up SD Armor Development Environment"

# Check if we're on a supported platform
PLATFORM=$(uname)
echo "🖥️  Detected platform: $PLATFORM"

# Install Rust if not present
if ! command -v rustc &> /dev/null; then
    echo "🦀 Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
else
    echo "✅ Rust is already installed"
fi

# Install Python if not present
if ! command -v python3 &> /dev/null; then
    echo "🐍 Python 3 is required but not found. Please install Python 3.8 or later."
    exit 1
else
    echo "✅ Python is already installed"
fi

# Setup Python virtual environment for backend
echo "🔧 Setting up Python virtual environment..."
cd SC_Server
python3 -m venv venv
source venv/bin/activate
pip install --upgrade pip
pip install -r requirements.txt
cd ..

# Setup Desktop client dependencies
echo "🔧 Setting up Desktop client dependencies..."
cd Desktop
cargo update
cd ..

# Create necessary directories
echo "📁 Creating necessary directories..."
mkdir -p logs
mkdir -p data/rules
mkdir -p data/reports

echo "✅ Development environment setup complete!"
echo ""
echo "To activate the Python virtual environment, run:"
echo "  cd SC_Server && source venv/bin/activate"
echo ""
echo "To build the Desktop client, run:"
echo "  cd Desktop && cargo build"