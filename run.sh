#!/bin/bash

# Dejavu API - Rust Quick Start Script

echo "🦀 Dejavu API - Rust Edition"
echo "=============================="
echo ""

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust is not installed!"
    echo "Please install Rust from: https://rustup.rs/"
    exit 1
fi

echo "✅ Rust is installed"
echo ""

# Check if .env exists
if [ ! -f .env ]; then
    echo "⚠️  .env file not found!"
    echo "Creating .env from .env.example..."
    cp .env.example .env
    echo "⚠️  Please edit .env with your MongoDB credentials before running!"
    exit 1
fi

echo "✅ .env file found"
echo ""

# Build and run
echo "🔨 Building project..."
cargo build

if [ $? -eq 0 ]; then
    echo ""
    echo "✅ Build successful!"
    echo ""
    echo "🚀 Starting server..."
    echo ""
    cargo run
else
    echo ""
    echo "❌ Build failed!"
    exit 1
fi
