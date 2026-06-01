#!/bin/bash

set -e

BINARY_NAME="vmctl"
INSTALL_DIR="$HOME/.local/bin"
SOURCE="./target/release/vmctl-rust"
CONFIG_DIR="$HOME/.config/vmctl"
LOGO_DEST="$CONFIG_DIR/logo.txt"
LOGO_SRC="./ascii.txt"

echo "Installing $BINARY_NAME..."

# 检查是否在项目目录
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Please run this script from the project root directory."
    exit 1
fi

# 构建发布版本（静默构建，失败则退出）
echo "Building release binary..."
if ! cargo build --release --quiet >/dev/null 2>&1; then
    echo "Error: cargo build --release failed."
    echo "请运行 'cargo build --release' 查看详细错误信息。"
    exit 1
fi

# 创建目标目录
mkdir -p "$INSTALL_DIR"

# 复制二进制文件
cp "$SOURCE" "$INSTALL_DIR/$BINARY_NAME"
chmod +x "$INSTALL_DIR/$BINARY_NAME"

echo "Installed to $INSTALL_DIR/$BINARY_NAME"

# 拷贝 logo 文件到用户配置目录
mkdir -p "$CONFIG_DIR"
if [ -f "$LOGO_SRC" ]; then
    cp "$LOGO_SRC" "$LOGO_DEST"
    echo "Logo copied to $LOGO_DEST"
else
    echo "WARNING: $LOGO_SRC not found, skipping logo copy."
fi

# 检查 PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo ""
    echo "WARNING: $INSTALL_DIR is not in your PATH."
    echo "Add this to your shell config (.zshrc, .bashrc, etc):"
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

# 输出环境变量配置
echo ""
echo "=========================================="
echo "请将以下环境变量加入你的 shell 配置文件"
echo "（如 ~/.zshrc 或 ~/.bashrc）："
echo "=========================================="
cat <<'EOF'

export VMCTL_VMRUN_PATH="/Applications/VMware Fusion.app/Contents/Library/vmrun"
export VMCTL_VM_DIR="$HOME/Virtual Machines.localized"
export VMCTL_REFRESH_INTERVAL=5
export VMCTL_LOGO_PATH="$HOME/.config/vmctl/logo.txt"

EOF
echo "=========================================="
echo "添加完成后，请记得执行以下命令使其生效："
echo "  source ~/.zshrc    # 如果使用 zsh"
echo "  source ~/.bashrc   # 如果使用 bash"
echo "=========================================="

echo ""
echo "Done!"
