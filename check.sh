#!/bin/bash
set -e

# ============================================================
# work-kb Rust 后端编译验证脚本
# 用法:  chmod +x check.sh && ./check.sh [项目根目录]
# 不传参数则默认当前目录为项目根目录
# ============================================================

PROJECT_DIR="${1:-$(pwd)}"
SRC_TAURI="$PROJECT_DIR/src-tauri"

if [ ! -f "$SRC_TAURI/Cargo.toml" ]; then
    echo "错误: 未找到 $SRC_TAURI/Cargo.toml"
    echo "请将项目 work-kb 目录上传到 Linux 后重试"
    exit 1
fi

# ---- 1. 安装 Rust（已安装则跳过）----
if ! command -v cargo &> /dev/null; then
    echo ">>> 安装 Rust 工具链..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi
echo ">>> Rust: $(rustc --version)"

# ---- 2. 安装 Tauri 2 系统依赖（Ubuntu/Debian）----
echo ">>> 安装 Tauri 2 系统依赖..."
sudo apt update -y
sudo apt install -y \
    build-essential \
    pkg-config \
    libwebkit2gtk-4.1-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    libssl-dev \
    libxdo-dev

# ---- 3. cargo check ----
cd "$SRC_TAURI"
echo ">>> 在 $(pwd) 执行 cargo check..."
cargo check 2>&1

echo ""
echo ">>> cargo check 完成 (退出码 $?)"
