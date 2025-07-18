# AI对话改进功能

## 功能概述

在 matecode 的 `commit` 命令中新增了 **AI对话改进** 功能，让用户可以通过自然语言与AI交流来改进生成的commit消息。

## 功能特点

### 1. 交互式对话
- 用户可以用自然语言描述对commit消息的改进建议
- AI会根据用户的反馈智能改进commit消息
- 支持多轮对话，用户可以持续改进直到满意

### 2. 用户友好的界面
- 使用 `dialoguer` 库提供美观的交互界面
- 清晰的选项菜单和提示信息
- 支持中文界面，符合项目的本地化需求

### 3. 灵活的选择机制
在查看改进后的commit消息时，用户可以选择：
- ✅ **使用改进后的版本** - 采用AI改进的版本
- 🔄 **继续改进** - 基于当前版本继续提出改进建议
- ↩️ **返回原始版本** - 放弃改进，回到原始版本

## 使用流程

1. 运行 `matecode commit` 命令
2. AI生成初始commit消息
3. 在选项菜单中选择 **💬 AI对话改进**
4. 输入改进建议，例如：
   - "让提交信息更简洁"
   - "添加更多技术细节"
   - "使用更专业的术语"
   - "符合conventional commits格式"
5. AI根据建议生成改进版本
6. 用户可以选择采用、继续改进或返回原始版本
7. 重复步骤4-6直到满意
8. 最终提交改进后的commit消息

## 技术实现

### 代码修改点

1. **选项菜单扩展** (`src/main.rs`)
   - 在原有的4个选项基础上新增"💬 AI对话改进"选项
   - 更新了对应的处理逻辑

2. **AI对话循环**
   - 实现了多轮对话机制
   - 每次改进都会基于当前版本进行优化
   - 支持错误处理和用户中断

3. **提示词优化**
   - 专门设计了用于改进commit消息的提示词
   - 强调保持简洁明了和符合conventional commits格式

### 依赖库使用

- `dialoguer::Input` - 用于获取用户输入
- `dialoguer::Select` - 用于选择菜单
- `dialoguer::Confirm` - 用于确认操作

## 示例对话

```
💬 请告诉我您希望如何改进这条提交信息: 让提交信息更简洁一些

🤖 正在根据您的反馈改进提交信息...

============================================================
改进后的提交信息:
feat: add user authentication
============================================================

您对改进后的提交信息满意吗？
  ✅ 使用改进后的版本
  🔄 继续改进  
  ↩️ 返回原始版本
```

## 优势

1. **提升用户体验** - 用户可以轻松地与AI交流改进commit消息
2. **保持一致性** - AI会确保改进后的消息符合项目规范
3. **支持个性化** - 用户可以根据自己的偏好调整消息风格
4. **多轮优化** - 支持持续改进直到用户满意
5. **错误处理** - 当AI调用失败时提供友好的错误提示

## 兼容性

- 完全兼容现有的commit流程
- 不影响其他功能的正常使用
- 可选功能，用户可以选择是否使用

这个功能大大增强了matecode的实用性，让用户能够更好地控制和优化生成的commit消息。