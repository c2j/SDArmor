#!/bin/bash

# SD Armor Development Server Starter
# This script starts the development server for SD Armor

set -e  # Exit on any error

echo "🚀 Starting SD Armor Development Server"

# Check if we're in the right directory
if [ ! -d "SC_Server" ]; then
    echo "❌ Error: SC_Server directory not found. Please run this script from the project root."
    exit 1
fi

# Start the backend server
echo "🔧 Starting backend server..."
cd SC_Server
source venv/bin/activate

# Export environment variables
export FLASK_APP=server.py
export FLASK_ENV=development
export DATABASE_URI=sqlite:///app.db

# Initialize database if it doesn't exist
if [ ! -f "app.db" ]; then
    echo "💾 Initializing database..."
    python init_db.py
fi

echo "🌐 Server starting on http://localhost:5000"
echo "⏹️  Press Ctrl+C to stop the server"
python server.py

cd ..