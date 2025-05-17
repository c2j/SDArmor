#!/bin/bash
# SDChat Security Scanner Server Run Script

# Exit on error
set -e

# Check Python version
python_version=$(python3 --version 2>&1 | awk '{print $2}')
echo "Using Python $python_version"

# Check if virtual environment exists
if [ ! -d "venv" ]; then
    echo "Creating virtual environment..."
    python3 -m venv venv
fi

# Activate virtual environment
source venv/bin/activate

# Install dependencies if needed
if [ ! -f ".deps_installed" ] || [ requirements.txt -nt ".deps_installed" ]; then
    echo "Installing dependencies..."
    pip install -r requirements.txt
    touch .deps_installed
fi

# Check if database exists and run initialization if needed
if [ ! -f "data/sdchat.db" ]; then
    echo "Initializing database..."
    python init_db.py
fi

# Start the server
echo "Starting SDChat Scanner Server..."
python server.py "$@"