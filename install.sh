#!/bin/bash
set -e

# 检测系统架构和操作系统
ARCH=$(uname -m)
OS=$(uname -s)

FILENAME=""
echo "正在检测系统信息..."

if [ "$OS" == "Darwin" ]; then
    if [ "$ARCH" = "x86_64" ]; then
        FILENAME="matecode-${VERSION}-x86_64-apple-darwin.tar.gz"
        echo "🖥️  检测到 Intel Mac (x86_64)"
    elif [ "$ARCH" = "arm64" ]; then
        FILENAME="matecode-${VERSION}-aarch64-apple-darwin.tar.gz"
        echo "🍎 检测到 Apple Silicon Mac (arm64)"
    else
        echo "❌ 不支持的 Mac 架构: $ARCH"
        exit 1
    fi
elif [ "$OS" == "Linux" ]; then
    if [ "$ARCH" = "x86_64" ]; then
        FILENAME="matecode-${VERSION}-x86_64-unknown-linux-gnu.tar.gz"
        echo "🐧  检测到 Linux (x86_64)"
    else
        echo "❌ 不支持的 Linux 架构: $ARCH"
        exit 1
    fi
else
    echo "❌ 不支持的操作系统: $OS"
    exit 1
fi

# 设置版本（您需要根据实际情况修改）
# 请确保此版本与您GitHub Release的Tag一致
VERSION="v0.0.1"
# 替换为您的实际仓库地址
REPO="liuwwang/matecode"

# 下载 URL
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${FILENAME}"

echo "📥 正在下载 ${FILENAME}..."
# 使用 curl 下载文件，-L 跟随重定向，-o 指定输出文件名
curl -L -o "/tmp/${FILENAME}" "$DOWNLOAD_URL"

echo "📂 正在解压..."
# 进入临时目录并解压
cd /tmp
# 根据文件扩展名选择解压命令
if [[ "$FILENAME" == *.tar.gz ]]; then
    tar -xzf "$FILENAME"
elif [[ "$FILENAME" == *.zip ]]; then
    unzip "$FILENAME"
else
    echo "❌ 未知的文件格式，无法解压: $FILENAME"
    exit 1
fi

echo "📦 正在安装到 /usr/local/bin/..."
# 移动编译好的二进制文件到 /usr/local/bin/，需要 sudo 权限
# 注意：这里假设解压后直接得到的是 'matecode' 或 'matecode.exe' 文件
# 如果解压后是其他目录结构，这里需要调整
if [ "$OS" == "Windows" ]; then
    # Windows 的安装逻辑可能不同，这里仅为示例，实际可能需要配置 PATH
    echo "Windows 安装逻辑待完善..."
else
    sudo mv matecode /usr/local/bin/
fi

echo "🧹 清理临时文件..."
rm "/tmp/${FILENAME}"
# 如果解压产生了其他临时文件或目录，也需要在此处清理

echo "✅ 安装完成！"
echo ""
echo "🚀 现在您可以使用以下命令："
echo "   matecode --help"
echo "   matecode init"
echo "   matecode commit"
