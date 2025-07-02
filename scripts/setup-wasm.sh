#!/bin/bash

# WASM ファイルを public ディレクトリにコピーするスクリプト

# スクリプトのディレクトリを取得
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

# ディレクトリの作成
echo "Creating public/wasm directory..."
mkdir -p "$PROJECT_ROOT/public/wasm"

# WASM ファイルのコピー
echo "Copying WASM files..."
if [ -d "$PROJECT_ROOT/src-wasm/pkg" ]; then
    cp "$PROJECT_ROOT/src-wasm/pkg/kinegraph_wasm_bg.wasm" "$PROJECT_ROOT/public/wasm/"
    cp "$PROJECT_ROOT/src-wasm/pkg/kinegraph_wasm.js" "$PROJECT_ROOT/public/wasm/"
    cp "$PROJECT_ROOT/src-wasm/pkg/kinegraph_wasm.d.ts" "$PROJECT_ROOT/public/wasm/"
    echo "WASM files copied successfully!"
else
    echo "Error: WASM build directory not found. Please build the WASM module first."
    echo "Run: cd src-wasm && ./build.sh"
    exit 1
fi

echo "Setup complete!"