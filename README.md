# matecode

matecode 是一个命令行工具，旨在帮助开发者根据 `git diff` 的内容，快速生成符合 [Conventional Commits](https://www.conventionalcommits.org/zh-cn/v1.0.0/) 规范的提交信息。同时，它也具备生成和发送工作日报的能力（此功能正在开发中）。

## ✨ 主要功能

- **提交信息生成**: 分析 `git diff` 结果，自动生成结构化的提交信息。
- **多种LLM支持**: 可配置使用 Gemini, OpenAI, Ollama 等多种大型语言模型服务。
- **跨平台**: 支持在 Windows, macOS, 和 Linux 上运行。
- **配置灵活**: 用户可以通过配置文件自定义模型参数和忽略文件。

## 🛠️ 安装

**重要提示**: 以下安装脚本中的仓库地址 `liuwwang/matecode` 已根据您提供的信息预设。如果您更改了仓库名，请务必同步修改脚本中的地址。

### 方法一: 一键安装脚本 (推荐)

**Linux / macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/liuwwang/matecode/main/scripts/install.sh | bash
```

**Windows (PowerShell):**
```powershell
iwr -useb https://raw.githubusercontent.com/liuwwang/matecode/main/scripts/install.ps1 | iex
```

### 方法二: 从 Release 页面下载

1.  访问 [**[请在这里填写 Release 页面的链接]**](https://github.com/liuwwang/matecode/releases) 页面。
2.  下载适用于您操作系统的最新版本二进制文件。
3.  将下载的文件重命名为 `matecode` (或 `matecode.exe`) 并移动到您的系统 `PATH` 路径下，以便全局调用。

### 方法三: 从源码构建

#### 前置要求

- [Rust](https://rustup.rs/) (版本 1.70.0 或更高)
- [Git](https://git-scm.com/)

#### 构建步骤

```bash
# 1. 克隆您的仓库
git clone https://github.com/liuwwang/matecode.git
cd matecode

# 2. 执行构建
cargo build --release

# 3. (可选) 将生成的可执行文件移动到系统 PATH
# 可执行文件位于 target/release/matecode (Windows 下为 matecode.exe)
```

## 🚀 快速开始

### 1. 初始化配置

首次使用前，请运行初始化命令。它会在您的用户配置目录下创建所需的文件。

```bash
matecode init
```

### 2. 配置 .env 文件

初始化后，请编辑配置文件目录下的 `.env` 文件，以设置您要使用的语言模型。

**配置示例 (Gemini):**
```env
LLM_PROVIDER="gemini"
GEMINI_API_KEY="[请在这里填写您的 Gemini API Key]"
# GEMINI_MODEL_NAME="gemini-1.5-pro-latest" # 可选，默认为此模型
```

**配置示例 (OpenAI):**
```env
LLM_PROVIDER="openai"
OPENAI_API_KEY="[请在这里填写您的 OpenAI API Key]"
# OPENAI_API_URL="https://api.openai.com/v1/chat/completions" # 可选
# OPENAI_MODEL_NAME="gpt-4-turbo" # 可选
```

**配置示例 (本地 Ollama):**
```env
LLM_PROVIDER="ollama"
# 对于 Ollama，API Key 不是必需的
OPENAI_API_KEY="ollama" 
OPENAI_API_URL="http://localhost:11434/v1/chat/completions"
# OPENAI_MODEL_NAME="llama3" # 请确保您本地已有此模型
```

### 3. 生成 Commit Message

```bash
# 1. 将您的代码更改添加到暂存区
git add .

# 2. 运行命令生成提交信息
matecode commit

# 3. (推荐) 使用管道直接提交
matecode commit | git commit -F -
```

## 📋 命令参考

| 命令 | 描述 |
| :--- | :--- |
| `matecode init` | 初始化配置，在用户目录下创建 `.env` 和 `.matecode-ignore` 文件。 |
| `matecode commit` | 基于暂存区的文件变更生成提交信息。 |
| `matecode report` | **(开发中)** 生成并发送工作日报。 |

## ⚙️ 配置文件

-   `.env`: 用于存放 LLM 提供商的 API Key 和其他敏感配置。
-   `.matecode-ignore`: 用于指定在生成提交信息时需要忽略的文件或目录，语法与 `.gitignore` 相同。

## 🤝 贡献

本项目欢迎各种形式的贡献。如果您有好的想法或发现了问题，请通过以下方式参与：

1.  **Fork** 本仓库。
2.  创建您的特性分支 (`git checkout -b feature/NewFeature`)。
3.  提交您的代码更改 (`git commit -m 'feat: Add some NewFeature'`)。
4.  将您的分支推送到您的 Fork (`git push origin feature/NewFeature`)。
5.  提交一个 **Pull Request**。

或者，您可以直接提交 [Issues](https://github.com/liuwwang/matecode/issues) 来报告 Bug 或提出功能建议。

## 📄 许可证

本项目采用 [MIT License](LICENSE) 开源。

---
*该 README 由 [matecode](https://github.com/liuwwang/matecode) 生成，并由 liuwwang 最后修订。*
