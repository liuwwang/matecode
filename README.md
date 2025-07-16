# matecode 🤖

`matecode` 是一个 AI 驱动的 CLI 工具，旨在自动化和增强您的 Git 工作流程。它可以为您生成富有表现力的提交信息、撰写工作日报，甚至审查您的代码。

## ✨ 功能

- **AI 生成提交信息**: 根据您的暂存更改 (git diff) 自动生成清晰、规范的 `git commit` 信息。
- **AI 生成工作日报**: 根据指定日期范围内的提交历史，自动汇总和生成工作日报。
- **AI 代码审查**: 对您的暂存代码进行智能审查，提供改进建议。
- **交互式操作**: 在提交前，您可以选择直接提交、编辑、重新生成或放弃 AI 生成的信息。
- **Git 钩子集成**: 通过 `post-commit` 钩子自动归档提交信息，为日报生成积累素材。
- **多模型支持**: 支持多种大语言模型，如 OpenAI 的 GPT 系列和 Google 的 Gemini。
- **可自定义提示词**: 提示词模板存储在配置目录中，用户可以根据需要自定义。

## 🚀 安装

*(安装说明待补充)*

## 🛠️ 配置

### 1. 初始化配置

运行以下命令来初始化配置，它会创建完整的配置目录结构：

```bash
matecode init
```

这将在 `~/.config/matecode/` 目录下创建：
- `config.toml` - 主配置文件
- `prompts/` - 提示词模板目录
- `history/` - 历史记录目录
- `.matecode-ignore` - 文件忽略规则

### 2. 配置 API 密钥

编辑 `~/.config/matecode/config.toml` 文件，设置您的 API 密钥：

```toml
# 选择默认的 LLM 提供商：'openai' 或 'gemini'
provider = "openai"

# 界面语言
language = "zh-CN"

[llm.openai]
api_key = "sk-your-api-key-here"
api_base = "http://10.63.8.6:8082/v1"  # 您的私有化部署地址
default_model = "qwen2.5-72b-instruct-001"
proxy = "socks5h://127.0.0.1:1080"     # 可选，代理设置

[llm.gemini]
api_key = "your-gemini-api-key-here"
default_model = "gemini-2.0-flash-exp"
proxy = "socks5h://127.0.0.1:1080"     # 可选，代理设置
```

### 3. 模型配置说明

**私有化部署模型**：
- 所有私有化部署的模型（如 Qwen、ChatGLM 等）会自动使用 `default` 配置
- 无需为每个模型单独配置参数
- 如果需要调整，可以在配置文件中添加：

```toml
[llm.openai.models.default]
max_tokens = 32768        # 上下文窗口大小
max_output_tokens = 4096  # 最大输出长度
reserved_tokens = 1000    # 预留 tokens
```

**Gemini 2.5 Flash**：
- 已预配置 Gemini 2.0 Flash Experimental 的参数
- 支持超大上下文窗口（1M tokens）

### 4. 自定义提示词（可选）

提示词模板存储在 `~/.config/matecode/prompts/` 目录中：

- `commit.toml` - 提交信息生成提示
- `review.toml` - 代码审查提示
- `report.toml` - 工作日报生成提示
- `summarize.toml` - 代码摘要提示
- `combine.toml` - 合并摘要提示

您可以根据需要编辑这些模板文件来自定义 AI 的行为。

### 5. 文件忽略规则

在生成项目上下文时，`matecode` 会自动忽略一些不必要的文件，支持以下忽略规则：

1. **项目 .gitignore**: 使用项目根目录下的 `.gitignore` 文件
2. **matecode 特定忽略**: 使用 `~/.config/matecode/.matecode-ignore` 文件

这样可以确保 AI 不会分析临时文件、日志文件、依赖包等不相关的内容，提高分析质量和速度。项目的 `.gitignore` 文件会处理项目特定的忽略规则，而 `.matecode-ignore` 文件可以添加一些通用的忽略模式。

## 📝 使用方法

### 生成 Commit Message

```bash
# 对已暂存的文件生成 commit message
matecode commit

# 暂存所有已跟踪的文件并生成 commit message
matecode commit --all
```

### 生成工作日报

```bash
# 生成今天的工作日报
matecode report

# 生成指定日期的工作日报
matecode report --since "yesterday"

# 生成指定日期范围内的工作日报
matecode report --since "2024-05-01" --until "2024-05-10"
```

### 代码审查

```bash
# 对已暂存的文件进行代码审查
matecode review
```

### Git 钩子

为了让 `matecode report` 能够获取到所有团队成员的提交，即使他们不使用 `matecode commit`，您可以在项目的 Git 仓库中安装一个钩子。

```bash
# 在当前仓库安装 post-commit 钩子
matecode install-hook
```

这个钩子会把每次提交的元信息（作者、项目名、消息）记录到 `~/.config/matecode/history/` 目录中。

## 🔧 配置说明

### 模型配置

每个模型都有以下配置选项：

- `max_tokens`: 模型的最大上下文长度
- `max_output_tokens`: 模型的最大输出长度
- `reserved_tokens`: 为系统提示和其他开销预留的 token 数量

### 代理设置

如果您需要通过代理访问 API，可以在配置文件中设置：

```toml
[llm.openai]
proxy = "socks5h://127.0.0.1:1080"  # SOCKS5 代理
# 或者
proxy = "http://127.0.0.1:8889"     # HTTP 代理
```

## 🤝 贡献

欢迎提交 PRs 和 issues！

## 📄 许可证

本项目使用 [MIT](LICENSE) 许可证。