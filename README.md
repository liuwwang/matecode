# 🤖 matecode

一个用来自动生成 Git Commit 和工作日报的 CLI 工具，支持多种 LLM 提供商。

## ✨ 特性

- 🚀 **智能提交信息生成**: 基于 Git diff 自动生成符合 Conventional Commits 规范的提交信息
- 🌐 **多 LLM 支持**: 支持 OpenAI、Gemini、Ollama 等多种 LLM 提供商
- 📱 **跨平台支持**: 支持 Windows、macOS、Linux 等主流操作系统
- 🎨 **美观的用户界面**: 彩色输出和进度指示器
- ⚙️ **灵活配置**: 支持多种配置方式和自定义忽略规则

## 🛠️ 安装

### 方法一：一键安装脚本（推荐）

**Linux/macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/yourusername/matecode/main/scripts/install.sh | bash
```

**Windows (PowerShell):**
```powershell
iwr -useb https://raw.githubusercontent.com/yourusername/matecode/main/scripts/install.ps1 | iex
```

### 方法二：从 Release 下载

1. 前往 [Releases](https://github.com/yourusername/matecode/releases) 页面
2. 下载对应平台的二进制文件：
   - **Windows**: `matecode-windows-x86_64.exe`
   - **macOS (Intel)**: `matecode-macos-x86_64`
   - **macOS (Apple Silicon)**: `matecode-macos-aarch64`
   - **Linux (x86_64)**: `matecode-linux-x86_64`
   - **Linux (ARM64)**: `matecode-linux-aarch64`

3. 重命名并移动到 PATH 中：

**Windows (PowerShell):**
```powershell
# 重命名文件
Rename-Item matecode-windows-x86_64.exe matecode.exe
# 移动到 PATH 中的目录，例如：
Move-Item matecode.exe C:\Windows\System32\
```

**macOS/Linux:**
```bash
# 重命名文件
mv matecode-macos-x86_64 matecode  # 或对应的文件名
# 添加执行权限
chmod +x matecode
# 移动到 PATH 中的目录
sudo mv matecode /usr/local/bin/
```

### 方法三：从源码构建

#### 前置要求

- [Rust](https://rustup.rs/) 1.70.0 或更高版本
- [Git](https://git-scm.com/)

#### 构建步骤

```bash
# 克隆仓库
git clone https://github.com/yourusername/matecode.git
cd matecode

# 使用构建脚本（推荐）
# Linux/macOS:
./scripts/build.sh release

# Windows:
scripts\build.bat release

# 或直接使用 Cargo
cargo build --release
```

构建完成后，二进制文件位于 `target/release/matecode`（Windows 下为 `matecode.exe`）。

## 🚀 快速开始

### 1. 初始化配置

```bash
matecode init
```

这会在以下位置创建配置文件：
- **Windows**: `%APPDATA%\matecode\`
- **macOS**: `~/Library/Application Support/matecode/`
- **Linux**: `~/.config/matecode/`

### 2. 配置 LLM 提供商

编辑配置目录中的 `.env` 文件：

#### 使用 Gemini (默认)
```env
LLM_PROVIDER="gemini"
GEMINI_API_KEY="your_gemini_api_key_here"
GEMINI_MODEL_NAME="gemini-1.5-pro-latest"
```

#### 使用 OpenAI
```env
LLM_PROVIDER="openai"
OPENAI_API_KEY="your_openai_api_key_here"
OPENAI_API_URL="https://api.openai.com/v1/chat/completions"
OPENAI_MODEL_NAME="gpt-4-turbo"
```

#### 使用 Ollama (本地)
```env
LLM_PROVIDER="ollama"
OPENAI_API_KEY="ollama"
OPENAI_API_URL="http://localhost:11434/v1/chat/completions"
OPENAI_MODEL_NAME="llama3"
```

### 3. 使用

```bash
# 暂存你的更改
git add .

# 生成并显示提交信息
matecode commit

# 如果满意，可以复制输出的信息手动提交
# 或者直接使用管道：
matecode commit | git commit -F -
```

## 📋 命令详解

### `matecode init`
初始化配置文件，创建 `.env` 和 `.matecode-ignore` 文件。

### `matecode commit`
根据暂存的更改生成提交信息。

**选项：**
- `-s, --scope <SCOPE>`: 添加作用域到提交信息

**示例：**
```bash
matecode commit --scope frontend
```

### `matecode report`
生成工作日报（功能开发中）。

**选项：**
- `-a, --author <AUTHOR>`: 指定作者

## ⚙️ 配置文件

### `.env` 文件
包含 LLM 提供商的配置信息。

### `.matecode-ignore` 文件
指定在生成提交信息时要忽略的文件模式，语法类似 `.gitignore`。

默认忽略：
```
*.lock
*.log
*.json
```

## 🔧 开发

### 项目结构
```
matecode/
├── src/
│   ├── main.rs          # 主入口
│   ├── cli.rs           # CLI 接口定义
│   ├── config.rs        # 配置管理
│   ├── git.rs           # Git 操作
│   ├── lib.rs           # 库入口
│   └── llm/             # LLM 集成
│       ├── mod.rs       # LLM 模块
│       ├── openai.rs    # OpenAI 集成
│       └── gemini.rs    # Gemini 集成
├── scripts/             # 构建脚本
│   ├── build.sh         # Linux/macOS 构建
│   ├── build.bat        # Windows 构建
│   ├── install.sh       # Linux/macOS 安装
│   └── install.ps1      # Windows 安装
├── .github/
│   └── workflows/
│       └── build.yml    # CI/CD 配置
├── build.rs             # 构建脚本
├── Cargo.toml           # 项目配置
└── README.md
```

### 本地开发

```bash
# 克隆仓库
git clone https://github.com/yourusername/matecode.git
cd matecode

# 运行开发版本
cargo run -- init
cargo run -- commit

# 运行测试
cargo test

# 代码格式化
cargo fmt

# 代码检查
cargo clippy
```

### 跨平台构建

使用提供的构建脚本可以轻松进行跨平台构建：

```bash
# Linux/macOS
./scripts/build.sh release

# Windows
scripts\build.bat release

# 指定目标平台
./scripts/build.sh release x86_64-pc-windows-gnu
```

## 🤝 贡献

欢迎贡献代码！请遵循以下步骤：

1. Fork 本仓库
2. 创建你的特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交你的更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启一个 Pull Request

## 📄 许可证

本项目采用 MIT 许可证。详情请见 [LICENSE](LICENSE) 文件。

## 🙏 致谢

- [clap](https://github.com/clap-rs/clap) - 命令行参数解析
- [tokio](https://github.com/tokio-rs/tokio) - 异步运行时
- [reqwest](https://github.com/seanmonstar/reqwest) - HTTP 客户端
- [colored](https://github.com/mackwic/colored) - 彩色终端输出
- [indicatif](https://github.com/console-rs/indicatif) - 进度指示器

## 📞 支持

如果你遇到任何问题或有建议，请：

1. 查看 [Issues](https://github.com/yourusername/matecode/issues) 页面
2. 创建新的 Issue
3. 或者发送邮件至 [your.email@example.com](mailto:your.email@example.com)