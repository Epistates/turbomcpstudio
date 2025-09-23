#!/bin/bash

# MCP Studio Build Script
set -e

echo "🔧 Building MCP Studio..."

# Check if pnpm is installed
if ! command -v pnpm &> /dev/null; then
    echo "❌ pnpm is required but not installed. Please install it first:"
    echo "   npm install -g pnpm"
    exit 1
fi

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust is required but not installed. Please install it first:"
    echo "   https://rustup.rs/"
    exit 1
fi

# Install frontend dependencies
echo "📦 Installing frontend dependencies..."
pnpm install

# Check TypeScript
echo "🔍 Type checking..."
pnpm run check

# Build Tauri application
echo "🚀 Building Tauri application..."
pnpm run tauri build

echo "✅ Build completed successfully!"
echo "📦 Built application can be found in src-tauri/target/release/"