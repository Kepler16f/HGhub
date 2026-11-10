#!/usr/bin/env bash
# HGhub macOS 本地一键打包构建脚本
# 支持目标平台: arm64 (Apple Silicon), x64 (Intel), universal (通用双架构), 或当前系统原生架构

set -euo pipefail

TARGET_ARCH="${1:-native}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "=================================================="
echo "          HGhub macOS 构建打包向导               "
echo "=================================================="

# 1. 系统检测
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "❌ 错误: 本地打包 macOS 应用需要在 macOS 系统下运行。"
  echo "提示: 如果你在 Windows / Linux 系统，可通过 GitHub Actions 在线自动打包 macOS 版本（详见 docs/macos-port.md）。"
  exit 1
fi

# 2. 依赖检测
command -v node >/dev/null 2>&1 || { echo "❌ 缺少 Node.js 环境，请先安装 Node.js (推荐 v20+)"; exit 1; }
command -v pnpm >/dev/null 2>&1 || { echo "❌ 缺少 pnpm，正在尝试通过 corepack 启用..."; corepack enable; }
command -v cargo >/dev/null 2>&1 || { echo "❌ 缺少 Rust/Cargo，请先运行: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"; exit 1; }

# 3. 依赖安装
if [[ ! -d "node_modules" ]]; then
  echo "📦 正在安装前端依赖..."
  pnpm install
fi

# 4. 根据入参确定 target
TARGET_FLAG=""
case "$TARGET_ARCH" in
  arm64|aarch64)
    rustup target add aarch64-apple-darwin
    TARGET_FLAG="--target aarch64-apple-darwin"
    echo "🎯 目标架构: Apple Silicon (aarch64-apple-darwin)"
    ;;
  x64|x86_64|intel)
    rustup target add x86_64-apple-darwin
    TARGET_FLAG="--target x86_64-apple-darwin"
    echo "🎯 目标架构: Intel (x86_64-apple-darwin)"
    ;;
  universal|all)
    rustup target add aarch64-apple-darwin
    rustup target add x86_64-apple-darwin
    TARGET_FLAG="--target universal-apple-darwin"
    echo "🎯 目标架构: Universal 通用双架构 (universal-apple-darwin)"
    ;;
  native)
    echo "🎯 目标架构: 当前原生架构 ($(uname -m))"
    ;;
  *)
    echo "❌ 未知目标架构: $TARGET_ARCH，支持的选项: arm64 | x64 | universal | native"
    exit 1
    ;;
esac

# 5. 执行 Tauri 构建
echo "🚀 开始构建应用与 DMG 安装包..."
pnpm tauri build --config src-tauri/tauri.macos.conf.json $TARGET_FLAG

echo ""
echo "=================================================="
echo "🎉 构建完成！产物路径如下："
echo "=================================================="
find src-tauri/target -name "*.dmg" -o -name "HGhub.app" | grep -v "/deps/" || true

echo ""
echo "💡 提示（macOS 未签名提示绕过）："
echo "由于个人本地开源构建未进行 Apple 开发者证书签名与公证，"
echo "首次运行若提示「应用已损坏」或「无法打开」，请在终端执行："
echo "  sudo xattr -rd com.apple.quarantine /Applications/HGhub.app"
echo "或者在「访达」中按住 Control 键并单击应用选择「打开」。"
echo "=================================================="
