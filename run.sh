#!/bin/bash
echo ""
echo "  AI Launcher - Building..."
echo ""
cargo build --release || { echo "Build failed! Install Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"; exit 1; }
echo ""
echo "  Starting AI Launcher..."
echo "  Swagger UI: http://localhost:8080/swagger-ui/"
echo ""
./target/release/ai_launcher
