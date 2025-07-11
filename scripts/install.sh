#!/bin/bash

# matecode 安装脚本 (Linux/macOS)
# 使用方法: curl -fsSL https://raw.githubusercontent.com/yourusername/matecode/main/scripts/install.sh | bash

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 配置
REPO="yourusername/matecode"
INSTALL_DIR="/usr/local/bin"
BINARY_NAME="matecode"

echo -e "${BLUE}🚀 开始安装 matecode...${NC}"

# 检测操作系统和架构
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case $OS in
    linux)
        OS_NAME="linux"
        ;;
    darwin)
        OS_NAME="macos"
        ;;
    *)
        echo -e "${RED}❌ 不支持的操作系统: $OS${NC}"
        exit 1
        ;;
esac

case $ARCH in
    x86_64)
        ARCH_NAME="x86_64"
        ;;
    aarch64|arm64)
        ARCH_NAME="aarch64"
        ;;
    *)
        echo -e "${RED}❌ 不支持的架构: $ARCH${NC}"
        exit 1
        ;;
esac

# 构建下载文件名
if [ "$OS_NAME" = "linux" ] && [ "$ARCH_NAME" = "x86_64" ]; then
    DOWNLOAD_NAME="matecode-linux-x86_64"
elif [ "$OS_NAME" = "linux" ] && [ "$ARCH_NAME" = "aarch64" ]; then
    DOWNLOAD_NAME="matecode-linux-aarch64"
elif [ "$OS_NAME" = "macos" ] && [ "$ARCH_NAME" = "x86_64" ]; then
    DOWNLOAD_NAME="matecode-macos-x86_64"
elif [ "$OS_NAME" = "macos" ] && [ "$ARCH_NAME" = "aarch64" ]; then
    DOWNLOAD_NAME="matecode-macos-aarch64"
else
    echo -e "${RED}❌ 不支持的平台组合: $OS_NAME-$ARCH_NAME${NC}"
    exit 1
fi

echo -e "${BLUE}📋 安装信息:${NC}"
echo -e "  操作系统: $OS_NAME"
echo -e "  架构: $ARCH_NAME"
echo -e "  文件名: $DOWNLOAD_NAME"
echo -e "  安装目录: $INSTALL_DIR"

# 获取最新版本
echo -e "${BLUE}🔍 获取最新版本信息...${NC}"
LATEST_VERSION=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST_VERSION" ]; then
    echo -e "${RED}❌ 无法获取最新版本信息${NC}"
    exit 1
fi

echo -e "${GREEN}✅ 最新版本: $LATEST_VERSION${NC}"

# 构建下载 URL
DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST_VERSION/$DOWNLOAD_NAME"

# 创建临时目录
TMP_DIR=$(mktemp -d)
trap "rm -rf $TMP_DIR" EXIT

echo -e "${BLUE}📥 下载 $DOWNLOAD_NAME...${NC}"
if ! curl -fsSL "$DOWNLOAD_URL" -o "$TMP_DIR/$BINARY_NAME"; then
    echo -e "${RED}❌ 下载失败${NC}"
    exit 1
fi

# 添加执行权限
chmod +x "$TMP_DIR/$BINARY_NAME"

# 检查是否需要 sudo
if [ -w "$INSTALL_DIR" ]; then
    SUDO=""
else
    SUDO="sudo"
    echo -e "${YELLOW}⚠️  需要管理员权限来安装到 $INSTALL_DIR${NC}"
fi

# 安装二进制文件
echo -e "${BLUE}📦 安装到 $INSTALL_DIR...${NC}"
$SUDO mv "$TMP_DIR/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"

# 验证安装
if command -v "$BINARY_NAME" &> /dev/null; then
    echo -e "${GREEN}✅ 安装成功!${NC}"
    echo -e "${GREEN}🎉 运行 '$BINARY_NAME --help' 查看使用帮助${NC}"
    echo -e "${GREEN}🔧 运行 '$BINARY_NAME init' 初始化配置${NC}"
else
    echo -e "${RED}❌ 安装失败，请检查 $INSTALL_DIR 是否在 PATH 中${NC}"
    exit 1
fi 