#!/bin/bash

# matecode 跨平台构建脚本 (Linux/macOS)
# 使用方法: ./scripts/build.sh [release|debug] [target]

set -e

# 默认配置
BUILD_TYPE="${1:-release}"
TARGET="${2:-}"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 开始构建 matecode...${NC}"

# 检查 Rust 是否安装
if ! command -v rustc &> /dev/null; then
    echo -e "${RED}❌ 错误: 未找到 Rust 编译器${NC}"
    echo -e "${YELLOW}请先安装 Rust: https://rustup.rs/${NC}"
    exit 1
fi

# 检查 Git 是否安装
if ! command -v git &> /dev/null; then
    echo -e "${RED}❌ 错误: 未找到 Git${NC}"
    echo -e "${YELLOW}请先安装 Git${NC}"
    exit 1
fi

# 显示构建信息
echo -e "${BLUE}📋 构建信息:${NC}"
echo -e "  构建类型: ${BUILD_TYPE}"
echo -e "  目标平台: ${TARGET:-当前平台}"
echo -e "  Rust 版本: $(rustc --version)"
echo -e "  操作系统: $(uname -s)"
echo -e "  架构: $(uname -m)"

# 构建命令
CARGO_CMD="cargo build"

if [ "$BUILD_TYPE" = "release" ]; then
    CARGO_CMD="$CARGO_CMD --release"
    echo -e "${GREEN}🔧 执行发布构建...${NC}"
else
    echo -e "${YELLOW}🔧 执行调试构建...${NC}"
fi

if [ -n "$TARGET" ]; then
    CARGO_CMD="$CARGO_CMD --target $TARGET"
    echo -e "${BLUE}🎯 目标平台: $TARGET${NC}"
fi

# 执行构建
echo -e "${BLUE}⚙️  运行: $CARGO_CMD${NC}"
$CARGO_CMD

# 构建成功
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ 构建成功!${NC}"
    
    # 显示二进制文件位置
    if [ "$BUILD_TYPE" = "release" ]; then
        if [ -n "$TARGET" ]; then
            BINARY_PATH="target/$TARGET/release/matecode"
        else
            BINARY_PATH="target/release/matecode"
        fi
    else
        if [ -n "$TARGET" ]; then
            BINARY_PATH="target/$TARGET/debug/matecode"
        else
            BINARY_PATH="target/debug/matecode"
        fi
    fi
    
    if [ -f "$BINARY_PATH" ]; then
        echo -e "${GREEN}📦 二进制文件位置: $BINARY_PATH${NC}"
        echo -e "${GREEN}📊 文件大小: $(du -h "$BINARY_PATH" | cut -f1)${NC}"
    fi
else
    echo -e "${RED}❌ 构建失败!${NC}"
    exit 1
fi

echo -e "${GREEN}🎉 构建完成!${NC}" 