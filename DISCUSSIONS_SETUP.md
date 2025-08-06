# GitHub Discussions 设置指南

本文档说明如何为 MateCode 项目启用和配置 GitHub Discussions 功能。

## 🚀 启用 Discussions

### 1. 在 GitHub 仓库中启用 Discussions

1. 前往您的 GitHub 仓库页面
2. 点击 "Settings" 标签页
3. 在左侧菜单中找到 "General" 部分
4. 向下滚动到 "Features" 部分
5. 勾选 "Discussions" 选项
6. 点击 "Save" 保存设置

### 2. 配置讨论分类

启用 Discussions 后，您可以在仓库的 "Discussions" 标签页中：

1. 点击 "Categories" 管理讨论分类
2. 创建以下分类：
   - 💡 功能请求
   - 🐛 Bug 报告  
   - ❓ 使用问题
   - 💬 经验分享
   - 🔧 技术讨论
   - 📢 公告

### 3. 设置讨论模板

将以下文件复制到您的仓库中：

```
.github/
├── discussions/
│   ├── templates/
│   │   ├── feature-request.md
│   │   ├── bug-report.md
│   │   ├── usage-question.md
│   │   └── experience-share.md
│   ├── categories.yml
│   └── welcome-post.md
```

### 4. 创建欢迎帖子

1. 在 Discussions 页面点击 "New discussion"
2. 选择 "📢 公告" 分类
3. 使用 `welcome-post.md` 的内容创建欢迎帖子
4. 将此帖子置顶

## 📋 讨论模板说明

### 功能请求模板
用于用户提出新功能建议，包含：
- 功能描述
- 问题陈述
- 解决方案
- 环境信息

### Bug 报告模板
用于报告问题和错误，包含：
- Bug 描述
- 复现步骤
- 环境信息
- 错误日志

### 使用问题模板
用于寻求帮助，包含：
- 问题描述
- 尝试过的操作
- 期望结果

### 经验分享模板
用于分享使用经验，包含：
- 分享主题
- 详细内容
- 最佳实践
- 效果评估

## 🎯 管理讨论

### 标记和分类
- 使用标签对讨论进行分类
- 标记已解决的问题
- 突出显示重要讨论

### 社区管理
- 及时回复用户问题
- 鼓励用户分享经验
- 维护友好的讨论氛围

### 内容审核
- 确保讨论内容相关
- 删除垃圾信息
- 引导用户使用正确的模板

## 📊 讨论统计

启用 Discussions 后，您可以：
- 查看讨论参与度
- 分析热门话题
- 了解用户需求
- 跟踪问题解决情况

## 🔗 相关链接

- [GitHub Discussions 文档](https://docs.github.com/en/discussions)
- [讨论模板最佳实践](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests)
- [社区管理指南](https://docs.github.com/en/communities)

## 💡 建议

1. **定期更新**: 保持讨论模板和分类的更新
2. **积极回应**: 及时回复用户的讨论
3. **收集反馈**: 利用讨论收集用户反馈
4. **社区建设**: 鼓励用户之间的互动
5. **文档同步**: 根据讨论更新项目文档

---

通过启用 GitHub Discussions，您可以为 MateCode 项目建立一个活跃的社区，促进用户交流、问题解决和功能改进。 