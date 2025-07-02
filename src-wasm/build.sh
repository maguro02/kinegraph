#!/bin/bash

# Kinegraph WASM Build Script
# Builds the WebGPU WASM module with proper optimizations

set -e

echo "Building Kinegraph WASM module..."

# Install wasm-pack if not already installed
if ! command -v wasm-pack &> /dev/null; then
    echo "Installing wasm-pack..."
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

# Clean previous builds
rm -rf pkg

# Build with wasm-pack
echo "Running wasm-pack build..."
wasm-pack build \
    --target web \
    --out-dir pkg \
    --release \
    --no-opt \
    -- \
    --features "console_error_panic_hook"

# Post-process the generated files
echo "Post-processing WASM files..."

# Add SharedArrayBuffer and crossOriginIsolated checks to the generated JS
cat >> pkg/kinegraph_wasm.js << 'EOF'

// Check for required features
export function checkRequiredFeatures() {
    const features = {
        webgpu: 'gpu' in navigator,
        offscreenCanvas: typeof OffscreenCanvas !== 'undefined',
        sharedArrayBuffer: typeof SharedArrayBuffer !== 'undefined',
        crossOriginIsolated: window.crossOriginIsolated === true,
    };
    
    const missing = Object.entries(features)
        .filter(([_, supported]) => !supported)
        .map(([feature]) => feature);
    
    if (missing.length > 0) {
        console.warn('Missing required features:', missing);
    }
    
    return features;
}

// Helper to create OffscreenCanvas from regular canvas
export function createOffscreenCanvas(canvas) {
    if (canvas.transferControlToOffscreen) {
        return canvas.transferControlToOffscreen();
    }
    throw new Error('transferControlToOffscreen not supported');
}
EOF

# Copy to the parent project's public directory
mkdir -p ../public/wasm
cp pkg/* ../public/wasm/

echo "Build complete! Files are in pkg/ and ../public/wasm/"
echo ""
echo "To use in your project:"
echo "1. Ensure your server has proper COOP/COEP headers for SharedArrayBuffer"
echo "2. Import from '/wasm/kinegraph_wasm.js'"
echo "3. Check feature support with checkRequiredFeatures()"