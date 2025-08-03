# matecode

!!! 这是一个个人用作辅助编码工作的工具，如果有问题可以提出来一起讨论.

一个基于 AI 的代码管理工具，支持自动生成提交信息、代码审查和工作报告。

[![Rust CI](https://github.com/liuwwang/matecode/actions/workflows/ci.yml/badge.svg)](https://github.com/liuwwang/matecode/actions/workflows/ci.yml)
[![Latest Release](https://img.shields.io/github/v/release/liuwwang/matecode)](https://github.com/liuwwang/matecode/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

一个基于 AI 的 CLI 工具，旨在自动化 Git 提交信息和工作日报的生成，并提供代码审查功能。

An AI-powered CLI tool to automate the generation of Git commit messages, work reports, and provide code reviews.

---

## 🚀 功能 / Features

-   **智能 Commit 信息生成**: 根据 `git diff` 的内容，自动生成符合规范的提交信息。
-   **自动化工作日报**: 根据指定时间范围内的 Git 提交历史，一键生成工作日报。
-   **AI 代码审查**: 对暂存区的代码改动进行智能审查，提出改进建议。
-   **多平台支持**: 支持 Windows, macOS, 和 Linux。
-   **高度可配置**: 支持 OpenAI、Gemini 等多种 LLM 服务商，并允许自定义 Prompt 模板。
-   **Git Hooks 集成**: 可作为 Git 的 `prepare-commit-msg` 钩子使用，无缝集成到您的工作流中。

## 📦 安装 / Installation

您可以从 [GitHub Releases](https://github.com/liuwwang/matecode/releases) 页面下载最新的预编译二进制文件。

1.  前往 [Releases 页面](https://github.com/liuwwang/matecode/releases/latest)。
2.  根据您的操作系统，下载对应的压缩包（例如 `matecode-v0.1.0-x86_64-unknown-linux-gnu.tar.gz`）。
3.  解压文件，得到可执行文件 `matecode` (或 `matecode.exe`)。
4.  将该文件移动到您的系统路径下，例如 `/usr/local/bin` (Linux/macOS) 或 `C:\Windows\System32` (Windows)，以便在任何地方都能调用它。

## 🛠️ 使用方法 / Usage

### 1. 初始化配置

在第一次使用前，运行初始化命令来生成默认的配置文件：

```bash
matecode init
```

该命令会在您的用户配置目录下创建 `matecode` 文件夹（例如 `~/.config/matecode`），并生成 `config.toml` 和 `prompts` 模板。

**重要提示**: 您需要根据提示，编辑 `config.toml` 文件并填入您的 LLM API Key。

### 2. 生成 Commit 信息

当您完成代码修改并使用 `git add` 将其暂存后，运行：

```bash
matecode commit
# 别名: matecode c
```

如果您想让工具自动暂存文件的变更，可以使用 `-a` 或 `--all` 参数。这个参数的行为类似于 `git add -u`：

```bash
matecode commit --all
```

**重要提示**: `-a` 参数只会暂存**已被 Git 跟踪**的文件的**修改**和**删除**。它**不会**暂存您新建的、尚未被跟踪的文件（untracked files）。

### 3. 生成工作日报

根据您的提交历史生成工作报告：

```bash
matecode report
# 别名: matecode r
```

默认情况下，它会生成当天的工作报告。您也可以指定时间范围：

```bash
# 生成过去7天的工作报告
matecode report --since "7d ago"

# 生成从2023年10月1日到10月31日的工作报告
matecode report --since "2023-10-01" --until "2023-10-31"
```

### 4. AI 代码审查

对您暂存区的代码进行一次快速的 AI 审查：

```bash
matecode review
# 别名: matecode rev
```

### 5. 安装 Git Hook

为了获得最佳体验，您可以将 `matecode` 安装为 Git 钩子。这样，在您每次运行 `git commit` 时，它都会自动为您生成提交信息。

```bash
matecode install-hook
```

## ⚙️ 配置 / Configuration

所有的配置都在 `config.toml` 文件中。

-   **`provider`**: 设置默认的 LLM 服务商，可选值为 `"openai"` 或 `"gemini"`。
-   **`language`**: 设置生成内容的语言，例如 `"zh-CN"` 或 `"en-US"`。
-   **`llm.openai` / `llm.gemini`**:
    -   `api_key`: **必需**，您的 API 密钥。
    -   `api_base`: 如果您使用自托管的服务或代理，请设置此项。
    -   `default_model`: 指定该服务商下使用的默认模型。
-   **`prompts` 目录**: 您可以修改 `prompts` 目录下的 `.toml` 文件来完全自定义生成内容时使用的提示词模板。

## 🧑‍💻 从源码构建 / Building From Source

如果您想自行编译项目：

1.  确保您已安装 [Rust](https://www.rust-lang.org/tools/install)。
2.  克隆本仓库：
    ```bash
    git clone https://github.com/liuwwang/matecode.git
    cd matecode
    ```
3.  编译项目：
    ```bash
    cargo build --release
    ```
4.  编译好的二进制文件将位于 `./target/release/matecode`。

## 📜 许可证 / License

本项目采用 [MIT](https://opensource.org/licenses/MIT) 许可证。
