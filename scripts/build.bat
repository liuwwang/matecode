@echo off
setlocal enabledelayedexpansion

REM matecode 跨平台构建脚本 (Windows)
REM 使用方法: scripts\build.bat [release|debug] [target]

set "BUILD_TYPE=%~1"
set "TARGET=%~2"

if "%BUILD_TYPE%"=="" set "BUILD_TYPE=release"

echo 🚀 开始构建 matecode...

REM 检查 Rust 是否安装
where rustc >nul 2>nul
if %errorlevel% neq 0 (
    echo ❌ 错误: 未找到 Rust 编译器
    echo 请先安装 Rust: https://rustup.rs/
    exit /b 1
)

REM 检查 Git 是否安装
where git >nul 2>nul
if %errorlevel% neq 0 (
    echo ❌ 错误: 未找到 Git
    echo 请先安装 Git
    exit /b 1
)

REM 显示构建信息
echo 📋 构建信息:
echo   构建类型: %BUILD_TYPE%
if "%TARGET%"=="" (
    echo   目标平台: 当前平台
) else (
    echo   目标平台: %TARGET%
)

for /f "tokens=*" %%i in ('rustc --version') do set "RUST_VERSION=%%i"
echo   Rust 版本: %RUST_VERSION%

for /f "tokens=*" %%i in ('ver') do set "OS_VERSION=%%i"
echo   操作系统: %OS_VERSION%

REM 构建命令
set "CARGO_CMD=cargo build"

if "%BUILD_TYPE%"=="release" (
    set "CARGO_CMD=!CARGO_CMD! --release"
    echo 🔧 执行发布构建...
) else (
    echo 🔧 执行调试构建...
)

if not "%TARGET%"=="" (
    set "CARGO_CMD=!CARGO_CMD! --target %TARGET%"
    echo 🎯 目标平台: %TARGET%
)

REM 执行构建
echo ⚙️  运行: !CARGO_CMD!
!CARGO_CMD!

if %errorlevel% equ 0 (
    echo ✅ 构建成功!
    
    REM 显示二进制文件位置
    if "%BUILD_TYPE%"=="release" (
        if "%TARGET%"=="" (
            set "BINARY_PATH=target\release\matecode.exe"
        ) else (
            set "BINARY_PATH=target\%TARGET%\release\matecode.exe"
        )
    ) else (
        if "%TARGET%"=="" (
            set "BINARY_PATH=target\debug\matecode.exe"
        ) else (
            set "BINARY_PATH=target\%TARGET%\debug\matecode.exe"
        )
    )
    
    if exist "!BINARY_PATH!" (
        echo 📦 二进制文件位置: !BINARY_PATH!
        for %%A in ("!BINARY_PATH!") do echo 📊 文件大小: %%~zA 字节
    )
    
    echo 🎉 构建完成!
) else (
    echo ❌ 构建失败!
    exit /b 1
)

endlocal 