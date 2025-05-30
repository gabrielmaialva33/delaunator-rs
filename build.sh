#!/bin/bash
set -e

# Ensure rustup Rust is used
export PATH="$HOME/.cargo/bin:$PATH"

echo "🔺 Building Delaunator-RS WebAssembly Package..."

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "❌ Error: wasm-pack is not installed."
    echo "📦 Install it with: cargo install wasm-pack"
    exit 1
fi

# Clean previous builds
echo "🧹 Cleaning previous builds..."
rm -rf pkg/ www/pkg/

# Build the WASM package with optimizations
echo "⚡ Building WebAssembly package with optimizations..."
wasm-pack build --target web --release --out-dir pkg

# Create www/pkg directory
echo "📁 Creating www/pkg directory..."
mkdir -p www/pkg

# Copy only necessary WASM files to www/pkg
echo "📋 Copying WASM files to www/pkg..."
cp pkg/delaunator_rs_bg.wasm www/pkg/
cp pkg/delaunator_rs.js www/pkg/

# Verify files were copied successfully
if [[ -f "www/pkg/delaunator_rs_bg.wasm" && -f "www/pkg/delaunator_rs.js" ]]; then
    echo "✅ Build completed successfully!"
    echo ""
    echo "🚀 To run the web demo:"
    echo "   cd www && python3 -m http.server 8080"
    echo "   Then open: http://localhost:8080"
else
    echo "❌ Build failed: WASM files not found"
    exit 1
fi
