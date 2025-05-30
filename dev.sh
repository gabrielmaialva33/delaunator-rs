#!/bin/bash

echo "🔺 Delaunator-RS Development Server"
echo "================================="

# Ensure rustup Rust is used
export PATH="$HOME/.cargo/bin:$PATH"

# Build the project
echo "🔨 Building project..."
./build.sh

if [ $? -eq 0 ]; then
    echo ""
    echo "🌐 Starting development server..."
    echo "📍 Server will be available at: http://localhost:8080"
    echo "🛑 Press Ctrl+C to stop the server"
    echo ""
    
    # Start the server in the www directory
    cd www && python3 -m http.server 8080
else
    echo "❌ Build failed. Please check the errors above."
    exit 1
fi