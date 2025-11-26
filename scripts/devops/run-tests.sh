#!/bin/bash

# SD Armor Test Runner Script
# This script runs all tests for SD Armor

set -e  # Exit on any error

echo "🧪 Running SD Armor Tests"

# Run backend tests
echo "🔧 Running backend tests..."
cd SC_Server
source venv/bin/activate

# Run unit tests
echo "📋 Running unit tests..."
python -m pytest tests/unit/ -v

# Run integration tests
echo "🔗 Running integration tests..."
python -m pytest tests/integration/ -v

# Run contract tests
echo "📝 Running contract tests..."
python -m pytest tests/contract/ -v

cd ..

# Run desktop client tests
echo "🖥️  Running desktop client tests..."
cd Desktop
cargo test
cd ..

echo "✅ All tests completed successfully!"