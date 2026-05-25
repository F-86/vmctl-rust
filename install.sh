#!/bin/bash

set -e

BINARY_NAME="vmctl"
INSTALL_DIR="$HOME/.local/bin"
SOURCE="./target/release/vmctl-rust"

echo "Installing $BINARY_NAME..."

# 检查是否在项目目录
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Please run this script from the project root directory."
    exit 1
fi

# 检查是否已构建
if [ ! -f "$SOURCE" ]; then
    echo "Binary not found. Building..."
    cargo build --release
fi

# 创建目标目录
mkdir -p "$INSTALL_DIR"

# 复制二进制文件
cp "$SOURCE" "$INSTALL_DIR/$BINARY_NAME"
chmod +x "$INSTALL_DIR/$BINARY_NAME"

echo "Installed to $INSTALL_DIR/$BINARY_NAME"

# 检查 PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo ""
    echo "WARNING: $INSTALL_DIR is not in your PATH."
    echo "Add this to your shell config (.zshrc, .bashrc, etc):"
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

echo "Done!"
