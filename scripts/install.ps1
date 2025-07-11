# matecode Windows 安装脚本
# 使用方法: iwr -useb https://raw.githubusercontent.com/yourusername/matecode/main/scripts/install.ps1 | iex

param(
    [string]$InstallDir = "$env:USERPROFILE\bin",
    [string]$Repo = "yourusername/matecode"
)

# 错误处理
$ErrorActionPreference = "Stop"

Write-Host "🚀 开始安装 matecode..." -ForegroundColor Blue

# 检测架构
$Arch = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { "x86" }
$DownloadName = "matecode-windows-$Arch.exe"

Write-Host "📋 安装信息:" -ForegroundColor Blue
Write-Host "  操作系统: Windows"
Write-Host "  架构: $Arch"
Write-Host "  文件名: $DownloadName"
Write-Host "  安装目录: $InstallDir"

# 获取最新版本
Write-Host "🔍 获取最新版本信息..." -ForegroundColor Blue
try {
    $LatestRelease = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
    $LatestVersion = $LatestRelease.tag_name
    Write-Host "✅ 最新版本: $LatestVersion" -ForegroundColor Green
} catch {
    Write-Host "❌ 无法获取最新版本信息: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

# 构建下载 URL
$DownloadUrl = "https://github.com/$Repo/releases/download/$LatestVersion/$DownloadName"

# 创建安装目录
if (-not (Test-Path $InstallDir)) {
    Write-Host "📁 创建安装目录: $InstallDir" -ForegroundColor Blue
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

# 下载文件
$TempFile = [System.IO.Path]::GetTempFileName() + ".exe"
$BinaryPath = Join-Path $InstallDir "matecode.exe"

Write-Host "📥 下载 $DownloadName..." -ForegroundColor Blue
try {
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $TempFile
    Write-Host "✅ 下载完成" -ForegroundColor Green
} catch {
    Write-Host "❌ 下载失败: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

# 移动文件到安装目录
Write-Host "📦 安装到 $InstallDir..." -ForegroundColor Blue
try {
    Move-Item $TempFile $BinaryPath -Force
    Write-Host "✅ 文件安装成功" -ForegroundColor Green
} catch {
    Write-Host "❌ 安装失败: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

# 检查 PATH 环境变量
$CurrentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($CurrentPath -notlike "*$InstallDir*") {
    Write-Host "🔧 添加到 PATH 环境变量..." -ForegroundColor Blue
    try {
        $NewPath = "$CurrentPath;$InstallDir"
        [Environment]::SetEnvironmentVariable("PATH", $NewPath, "User")
        Write-Host "✅ PATH 环境变量已更新" -ForegroundColor Green
        Write-Host "⚠️  请重新启动 PowerShell 或命令提示符以使 PATH 生效" -ForegroundColor Yellow
    } catch {
        Write-Host "❌ 无法更新 PATH 环境变量: $($_.Exception.Message)" -ForegroundColor Red
        Write-Host "请手动将 '$InstallDir' 添加到 PATH 环境变量" -ForegroundColor Yellow
    }
} else {
    Write-Host "✅ PATH 环境变量已包含安装目录" -ForegroundColor Green
}

# 验证安装
Write-Host "🧪 验证安装..." -ForegroundColor Blue
try {
    $Version = & $BinaryPath --version
    Write-Host "✅ 安装成功!" -ForegroundColor Green
    Write-Host "🎉 运行 'matecode --help' 查看使用帮助" -ForegroundColor Green
    Write-Host "🔧 运行 'matecode init' 初始化配置" -ForegroundColor Green
} catch {
    Write-Host "❌ 验证失败，但文件已安装到 $BinaryPath" -ForegroundColor Red
    Write-Host "请确保 $InstallDir 在 PATH 环境变量中" -ForegroundColor Yellow
}

Write-Host "🎉 安装完成!" -ForegroundColor Green 