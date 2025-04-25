#!/bin/bash
set -e

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "Error: wasm-pack is not installed. Please install it with 'cargo install wasm-pack'."
    exit 1
fi

# Build the WASM package
echo "Building WebAssembly package..."
wasm-pack build --target web

# Create a www/pkg directory
mkdir -p www/pkg

# Copy the WASM files to www/pkg
echo "Copying WASM files to www/pkg directory..."
cp pkg/delaunator_rs_bg.wasm www/pkg/
cp pkg/delaunator_rs.js www/pkg/

echo "Build completed successfully!"
echo "To run the web demo, serve the www directory with a local server."
echo "For example: python3 -m http.server --directory www"
