#!/bin/bash
set -e

# --- Configuration ---
VERSION="v0.0.1" # Replace with your actual version
REPO="liuwwang/matecode" # Replace with your actual GitHub repository

# --- Helper Functions ---
print_info() {
    echo "ℹ️  $1"
}

print_success() {
    echo "✅ $1"
}

print_warning() {
    echo "⚠️ $1"
}

print_error() {
    echo "❌ $1" >&2
    exit 1
}

# --- Detect System ---
OS=$(uname -s)
ARCH=$(uname -m)

INSTALL_DIR="/usr/local/bin" # Default installation directory

# --- Installation Logic ---
if [ "$OS" = "Darwin" ]; then
    print_info "检测到 macOS 系统 ($ARCH)"
    if [ "$ARCH" = "x86_64" ]; then
        FILENAME="matecode-${VERSION}-x86_64-apple-darwin.tar.gz"
    elif [ "$ARCH" = "arm64" ]; then
        FILENAME="matecode-${VERSION}-aarch64-apple-darwin.tar.gz"
    else
        print_error "不支持的 macOS 架构: $ARCH"
    fi
    BINARY_NAME="matecode"

elif [ "$OS" = "Linux" ]; then
    print_info "检测到 Linux 系统 ($ARCH)"
    if [ "$ARCH" = "x86_64" ]; then
        FILENAME="matecode-${VERSION}-x86_64-unknown-linux-gnu.tar.gz"
    # Removed ARM Linux support as requested
    # elif [ "$ARCH" = "aarch64" ]; then
    #     FILENAME="matecode-${VERSION}-aarch64-unknown-linux-gnu.tar.gz"
    else
        print_error "不支持的 Linux 架构: $ARCH"
    fi
    BINARY_NAME="matecode"

elif [[ "$OS" == CYGWIN* || "$OS" == MINGW* || "$OS" == MSYS* ]]; then
    print_info "检测到 Windows 环境 ($ARCH)"
    # Note: Shell scripts on Windows often run in environments like Git Bash or WSL.
    # This part assumes a Unix-like environment with curl, tar, unzip, and mv.
    # For native Windows cmd/PowerShell, a different approach (e.g., .exe installer) is needed.
    if [ "$ARCH" = "x86_64" ]; then
        FILENAME="matecode-${VERSION}-x86_64-pc-windows-msvc.zip" # Assuming .zip for Windows
    # Removed ARM Windows support as requested
    # elif [ "$ARCH" = "aarch64" ]; then
    #     FILENAME="matecode-${VERSION}-aarch64-pc-windows-msvc.zip"
    else
        print_error "不支持的 Windows 架构: $ARCH"
    fi
    BINARY_NAME="matecode.exe"
    INSTALL_DIR="$HOME/bin" # Suggest installing in user's home bin directory
    print_warning "将在 $INSTALL_DIR 安装。请确保此目录在您的 PATH 环境变量中。"

else
    print_error "不支持的操作系统: $OS"
fi

# --- Download and Install ---
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${FILENAME}"
TEMP_FILE="/tmp/${FILENAME}"

print_info "正在下载 ${FILENAME}..."
if ! curl -L -o "$TEMP_FILE" "$DOWNLOAD_URL"; then
    print_error "下载失败。请检查 URL: $DOWNLOAD_URL 是否正确，以及网络连接。"
fi

print_info "正在解压..."
cd /tmp
if [[ "$FILENAME" == *.tar.gz ]]; then
    if ! tar -xzf "$FILENAME"; then
        print_error "解压 .tar.gz 文件失败。"
    fi
elif [[ "$FILENAME" == *.zip ]]; then
    if ! unzip "$FILENAME"; then
        print_error "解压 .zip 文件失败。请确保已安装 unzip (例如: sudo apt install unzip 或 brew install unzip)。"
    fi
else
    print_error "不支持的文件格式: $FILENAME"
fi

# Ensure the binary is in the extracted directory with the correct name
# This might need adjustment based on how the archive is structured
if [ ! -f "$BINARY_NAME" ]; then
    # Try to find the binary if it's in a subdirectory
    FOUND_BINARY=$(find . -maxdepth 2 -name "$BINARY_NAME" -type f -print -quit)
    if [ -n "$FOUND_BINARY" ]; then
        mv "$FOUND_BINARY" .
    else
        print_error "在解压后的目录中未找到预期的二进制文件 '$BINARY_NAME'。"
    fi
fi

print_info "正在安装到 $INSTALL_DIR/..."
# Create install directory if it doesn't exist (especially for Windows $HOME/bin)
if [ ! -d "$INSTALL_DIR" ]; then
    print_info "创建安装目录 $INSTALL_DIR..."
    mkdir -p "$INSTALL_DIR"
fi

# Use sudo for system-wide install, or just mv for user-specific install
if [ "$INSTALL_DIR" = "/usr/local/bin" ]; then
    if ! sudo mv "$BINARY_NAME" "$INSTALL_DIR/"; then
        print_error "移动文件到 $INSTALL_DIR/ 时出错。请检查权限。"
    fi
else
    # For user-specific install (like $HOME/bin)
    if ! mv "$BINARY_NAME" "$INSTALL_DIR/"; then
        print_error "移动文件到 $INSTALL_DIR/ 时出错。"
    fi
fi

print_info "正在设置可执行权限..."
if [ "$INSTALL_DIR" = "/usr/local/bin" ]; then
    if ! sudo chmod +x "$INSTALL_DIR/$BINARY_NAME"; then
        print_error "设置可执行权限失败。"
    fi
else
    # For user-specific install, permissions might be set by default or need user intervention
    if ! chmod +x "$INSTALL_DIR/$BINARY_NAME"; then
        print_warning "设置可执行权限失败。请手动在终端执行: chmod +x $INSTALL_DIR/$BINARY_NAME"
    fi
fi

print_info "清理临时文件..."
rm "$TEMP_FILE"
# Clean up extracted files if they are not the binary itself
# This part is tricky as archive contents can vary. We'll try to remove the extracted binary if it was in a dir.
if [ -d "$BINARY_NAME" ]; then # If binary was extracted into a directory named after it
    rm -rf "$BINARY_NAME"
elif [ -f "$BINARY_NAME" ]; then # If binary was extracted directly
    # If the binary was extracted directly, we don't need to remove a directory.
    # If it was in a subdirectory, the find command above should have moved it.
    # This cleanup might need refinement based on actual archive structures.
    : # No-op, assuming the binary itself is what we want to keep.
fi


print_success "安装完成！"
echo ""
echo "🚀 现在您可以使用以下命令："
echo "   $BINARY_NAME --help"
echo "   $BINARY_NAME init"
echo "   $BINARY_NAME commit"

# Add to PATH instruction for Windows user-specific install
if [[ "$OS" == CYGWIN* || "$OS" == MINGW* || "$OS" == MSYS* ]] && [ "$INSTALL_DIR" = "$HOME/bin" ]; then
    echo ""
    echo "⚠️ 重要提示 (Windows 用户):"
    echo "   为了能在任何目录下直接运行 '$BINARY_NAME' 命令，请确保 '$INSTALL_DIR' 目录已添加到您的系统 PATH 环境变量中。"
    echo "   通常，您需要编辑您的 shell 配置文件（如 ~/.bashrc, ~/.zshrc, 或 Windows 的环境变量设置）来添加 '$INSTALL_DIR'。"
fi

exit 0
