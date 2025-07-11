# 更新日志

本文档记录了 matecode 项目的所有重要更改。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)，
并且本项目遵循 [语义化版本](https://semver.org/lang/zh-CN/) 规范。

## [未发布]

### 新增
- 🚀 跨平台兼容性改进
  - 改进了 Git 命令调用的跨平台兼容性
  - 优化了配置文件路径，遵循各平台标准
  - 添加了跨平台构建脚本
  - 完善了 CI/CD 配置，支持多平台自动构建

### 改进
- 📱 配置目录现在遵循平台标准：
  - Windows: `%APPDATA%\matecode\`
  - macOS: `~/Library/Application Support/matecode/`
  - Linux: `~/.config/matecode/`
- 🔧 改进了环境变量加载逻辑
- 📦 优化了构建配置和二进制文件大小
- 📚 完善了项目文档

### 修复
- 🐛 修复了 Windows 上 Git 命令调用可能失败的问题
- 🔧 修复了配置文件路径在不同平台上的兼容性问题

## [0.1.0] - 2024-01-XX

### 新增
- 🎉 项目初始版本
- 🤖 支持基于 Git diff 自动生成提交信息
- 🌐 支持多种 LLM 提供商：
  - OpenAI GPT 系列
  - Google Gemini
  - Ollama（本地部署）
- 📋 命令行界面：
  - `matecode init` - 初始化配置
  - `matecode commit` - 生成提交信息
  - `matecode report` - 生成工作日报（开发中）
- ⚙️ 灵活的配置系统
- 🎨 美观的用户界面和进度指示器
- 📝 支持 Conventional Commits 规范 