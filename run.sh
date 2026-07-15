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

# Start persistent similarity service in the background
echo "🧠 Starting semantic similarity service..."
python3 src/core/scripts/semantic_similarity_service.py &
SIMILARITY_PID=$!

# Ensure the service is stopped when the script exits
cleanup() {
    echo ""
    echo "🛑 Stopping semantic similarity service (PID $SIMILARITY_PID)..."
    kill $SIMILARITY_PID 2>/dev/null || true
}
trap cleanup EXIT

# Wait for the service to be ready
for i in {1..30}; do
    if curl -s http://127.0.0.1:8002/health >/dev/null 2>&1; then
        echo "✅ Semantic similarity service is ready"
        break
    fi
    sleep 1
done

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
