#!/bin/bash
set -e

# 检测系统架构
ARCH=$(uname -m)
OS=$(uname -s)

if [ "$OS" != "Darwin" ]; then
    echo "❌ 此脚本仅支持 macOS 系统"
    exit 1
fi

# 设置版本（您需要根据实际情况修改）
VERSION="v0.0.1"
REPO="liuwwang/matecode"  # 替换为您的实际仓库地址

# 根据架构选择下载文件
if [ "$ARCH" = "x86_64" ]; then
    FILENAME="matecode-${VERSION}-x86_64-apple-darwin.tar.gz"
    echo "🖥️  检测到 Intel Mac (x86_64)"
elif [ "$ARCH" = "arm64" ]; then
    FILENAME="matecode-${VERSION}-aarch64-apple-darwin.tar.gz"
    echo "🍎 检测到 Apple Silicon Mac (arm64)"
else
    echo "❌ 不支持的架构: $ARCH"
    exit 1
fi

# 下载 URL
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${FILENAME}"

echo "📥 正在下载 ${FILENAME}..."
curl -L -o "/tmp/${FILENAME}" "$DOWNLOAD_URL"

echo "📂 正在解压..."
cd /tmp
tar -xzf "$FILENAME"

echo "📦 正在安装到 /usr/local/bin/..."
sudo mv matecode /usr/local/bin/

sudo chomd +x /usr/local/bin/matecode

echo "🧹 清理临时文件..."
rm "/tmp/${FILENAME}"

echo "✅ 安装完成！"
echo ""
echo "🚀 现在您可以使用以下命令："
echo "   matecode --help"
echo "   matecode init"
echo "   matecode commit" 