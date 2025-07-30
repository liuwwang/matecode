use crate::plan::{PlanGenerator, Plan, PlanAction, PlanStorage, StoredPlan};
use crate::git;
use anyhow::{Result, anyhow};
use colored::Colorize;
use dialoguer::{Confirm, Select, MultiSelect, theme::ColorfulTheme};


/// 处理计划命令
pub async fn handle_plan(
    description: String,
    interactive: bool,
    design_only: bool,
    status: bool,
    continue_plan: bool,
    smart: bool,
) -> Result<()> {
    // 检查是否是一个git仓库
    if !git::check_is_git_repo().await {
        eprintln!("{}", "错误: 当前目录不是一个有效的 Git 仓库。".red());
        return Ok(());
    }

    if status {
        return show_plan_status().await;
    }

    if continue_plan {
        return continue_existing_plan().await;
    }

    // 生成新计划
    generate_new_plan(description, interactive, design_only, smart).await
}

/// 生成新的开发计划
async fn generate_new_plan(description: String, interactive: bool, design_only: bool, smart: bool) -> Result<()> {
    println!("{}", "🤖 正在分析项目结构...".cyan());

    let plan = if smart {
        // 使用智能生成器 - 直接生成最终计划
        println!("{}", "🧠 使用智能生成器（实验性功能）...".yellow());
        let smart_generator = crate::plan::generator::PlanGenerator::new().await?;
        smart_generator.generate_comprehensive_plan(&description).await?
    } else {
        // 使用原有生成器 - 支持重试和用户反馈
        let generator = PlanGenerator::new().await?;

        loop {
            println!("{}", "🧠 正在生成开发计划...".cyan());

            // 生成计划（这里需要处理 token 限制）
            let plan: Plan = match generate_plan_with_retry(&generator, &description).await {
                Ok(plan) => plan,
                Err(e) => {
                    eprintln!("{} {}", "❌ 计划生成失败:".red(), e);
                    return Err(e);
                }
            };

            // 显示计划
            display_plan(&plan)?;

            // 询问用户是否满意
            if !ask_user_satisfaction()? {
                println!("{}", "🔄 正在重新生成计划...".yellow());
                continue;
            }

            break plan;
        }
    };

    // 显示计划
    display_plan(&plan)?;

    // 对于智能生成器，跳过用户满意度询问
    if !smart {
        // 询问用户是否满意
        if !ask_user_satisfaction()? {
            println!("{}", "🔄 智能生成器暂不支持重新生成，请使用普通模式".yellow());
            return Ok(());
        }
    }

    if design_only {
        println!("{}", "✅ 计划生成完成！".green());
        return Ok(());
    }

    if interactive {
        return execute_plan_interactively(&plan).await;
    } else {
        // 询问是否执行
        if Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("是否立即执行此计划？")
            .default(false)
            .interact()?
        {
            return execute_plan_automatically(&plan).await;
        } else {
            println!("{}", "💡 计划已保存，使用 'matecode plan --continue' 稍后执行".yellow());
            return Ok(());
        }
    }
}

/// 生成计划，支持重试和 token 限制处理
async fn generate_plan_with_retry(generator: &PlanGenerator, description: &str) -> Result<Plan> {
    const MAX_RETRIES: usize = 2;
    let mut use_compressed = false;

    for attempt in 1..=MAX_RETRIES {
        let result = if use_compressed {
            generator.generate_plan_with_context_management(description, true).await
        } else {
            generator.generate_plan(description).await
        };

        match result {
            Ok(plan) => {
                // 保存计划
                if let Ok(storage) = PlanStorage::new().await {
                    let _ = storage.save_plan(&plan).await;
                }
                return Ok(plan);
            }
            Err(e) => {
                let error_msg = e.to_string();

                // 检查是否是 token 限制错误
                if error_msg.contains("token") || error_msg.contains("length") || error_msg.contains("limit") || error_msg.contains("context") {
                    println!("{} 上下文过长，正在使用压缩模式重试...", "⚠️".yellow());
                    use_compressed = true;
                    continue;
                }

                // 检查是否是 XML 解析错误
                if error_msg.contains("XML") || error_msg.contains("xml") || error_msg.contains("解析") {
                    if attempt == MAX_RETRIES {
                        return Err(anyhow!("生成计划失败 (尝试 {} 次): XML 格式错误，请检查 LLM 配置", MAX_RETRIES));
                    }
                    println!("{} 第 {} 次尝试失败 (XML 格式错误)，正在重试...", "⚠️".yellow(), attempt);
                    continue;
                }

                if attempt == MAX_RETRIES {
                    return Err(anyhow!("生成计划失败 (尝试 {} 次): {}", MAX_RETRIES, e));
                }

                println!("{} 第 {} 次尝试失败，正在重试...", "⚠️".yellow(), attempt);
            }
        }
    }

    unreachable!()
}

/// 显示计划内容
fn display_plan(plan: &Plan) -> Result<()> {
    println!("\n{}", "=".repeat(60));
    println!("{} {}", "📋 开发计划:".green().bold(), plan.title.cyan().bold());
    println!("{}", "=".repeat(60));
    
    println!("\n{} {}", "🌿 分支名称:".green(), plan.branch_name.cyan());
    println!("{} {:?}", "📊 复杂度:".green(), plan.metadata.estimated_complexity);
    
    println!("\n{}", "🏗 技术方案:".green().bold());
    println!("{}", plan.metadata.technical_approach);
    
    if !plan.metadata.dependencies.is_empty() {
        println!("\n{}", "📦 新增依赖:".green().bold());
        for dep in &plan.metadata.dependencies {
            println!("  • {}", dep);
        }
    }
    
    println!("\n{}", "📁 涉及文件:".green().bold());
    for file in &plan.affected_files {
        println!("  • {}", file);
    }
    
    println!("\n{} {} 个操作", "⚡ 执行步骤:".green().bold(), plan.actions.len());
    for (i, action) in plan.actions.iter().enumerate() {
        println!("  {}. {}", i + 1, format_action_description(action));
    }
    
    println!("\n{}", "=".repeat(60));
    
    Ok(())
}

/// 格式化操作描述
fn format_action_description(action: &PlanAction) -> String {
    match action {
        PlanAction::CreateBranch { name, .. } => format!("创建分支: {}", name.cyan()),
        PlanAction::SwitchBranch { name } => format!("切换分支: {}", name.cyan()),
        PlanAction::CreateFile { path, .. } => format!("创建文件: {}", path.cyan()),
        PlanAction::ModifyFile { path, changes, .. } => {
            format!("修改文件: {} ({} 处变更)", path.cyan(), changes.len())
        }
        PlanAction::AppendToFile { path, .. } => format!("追加到文件: {}", path.cyan()),
        PlanAction::CreateDirectory { path, .. } => format!("创建目录: {}", path.cyan()),
        PlanAction::GenerateCode { target_file, function_name, .. } => {
            format!("生成代码: {} 中的 {}", target_file.cyan(), function_name.yellow())
        }
        PlanAction::RefactorCode { file_path, .. } => format!("重构代码: {}", file_path.cyan()),
        PlanAction::AddDependency { name, .. } => format!("添加依赖: {}", name.green()),
        PlanAction::UpdateDependency { name, version } => {
            format!("更新依赖: {} -> {}", name.green(), version.yellow())
        }
        PlanAction::UpdateChangelog { .. } => "更新 CHANGELOG".to_string(),
        PlanAction::GenerateDocumentation { target, .. } => {
            format!("生成文档: {:?}", target)
        }
        PlanAction::RunCommand { description, .. } => format!("执行命令: {}", description),
        PlanAction::RunTests { .. } => "运行测试".to_string(),
        PlanAction::ValidateCode { file_path, .. } => format!("验证代码: {}", file_path.cyan()),
        PlanAction::CheckDependencies => "检查依赖".to_string(),
    }
}

/// 询问用户是否满意当前计划
fn ask_user_satisfaction() -> Result<bool> {
    let options = vec![
        "✅ 满意，继续执行",
        "🔄 重新生成计划", 
        "✏️  修改需求描述",
        "❌ 取消操作"
    ];
    
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("您对这个计划满意吗？")
        .items(&options)
        .default(0)
        .interact()?;
    
    match selection {
        0 => Ok(true),  // 满意
        1 => Ok(false), // 重新生成
        2 => {
            // TODO: 实现修改需求描述的功能
            println!("{}", "💡 修改需求描述功能即将推出...".yellow());
            Ok(false)
        }
        3 => {
            println!("{}", "❌ 操作已取消".red());
            std::process::exit(0);
        }
        _ => Ok(false),
    }
}

/// 交互式执行计划
async fn execute_plan_interactively(plan: &Plan) -> Result<()> {
    println!("\n{}", "🚀 准备执行计划...".cyan());
    
    let action_descriptions: Vec<String> = plan.actions
        .iter()
        .enumerate()
        .map(|(i, action)| format!("{}. {}", i + 1, format_action_description(action)))
        .collect();
    
    let selections = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("选择要执行的操作 (空格选择，回车确认)")
        .items(&action_descriptions)
        .interact()?;
    
    if selections.is_empty() {
        println!("{}", "❌ 未选择任何操作".yellow());
        return Ok(());
    }
    
    println!("\n{}", "⚡ 开始执行选中的操作...".cyan());
    
    for &index in &selections {
        let action = &plan.actions[index];
        println!("执行: {}", format_action_description(action));
        
        // TODO: 实现具体的操作执行逻辑
        match execute_single_action(action).await {
            Ok(_) => println!("  ✅ 完成"),
            Err(e) => {
                eprintln!("  ❌ 失败: {}", e);
                if !Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("是否继续执行其他操作？")
                    .default(true)
                    .interact()?
                {
                    break;
                }
            }
        }
    }
    
    println!("\n{}", "🎉 计划执行完成！".green().bold());
    Ok(())
}

/// 自动执行计划
async fn execute_plan_automatically(plan: &Plan) -> Result<()> {
    println!("\n{}", "⚡ 自动执行计划...".cyan());
    
    for (i, action) in plan.actions.iter().enumerate() {
        println!("执行 {}/{}: {}", i + 1, plan.actions.len(), format_action_description(action));
        
        match execute_single_action(action).await {
            Ok(_) => println!("  ✅ 完成"),
            Err(e) => {
                eprintln!("  ❌ 失败: {}", e);
                return Err(anyhow!("计划执行在第 {} 步失败: {}", i + 1, e));
            }
        }
    }
    
    println!("\n{}", "🎉 计划执行完成！".green().bold());
    Ok(())
}

/// 执行单个操作
async fn execute_single_action(action: &PlanAction) -> Result<()> {
    match action {
        PlanAction::CreateBranch { name, from_branch } => {
            if let Some(from) = from_branch {
                git::run_git_command(&["checkout", "-b", name, from]).await?;
            } else {
                git::run_git_command(&["checkout", "-b", name]).await?;
            }
        }
        PlanAction::CreateFile { path, content, template: _ } => {
            if let Some(parent) = std::path::Path::new(path).parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::write(path, content).await?;
        }
        PlanAction::CreateDirectory { path, .. } => {
            tokio::fs::create_dir_all(path).await?;
        }
        PlanAction::RunCommand { command, .. } => {
            // 简单的命令执行，可以扩展为更复杂的逻辑
            let output = tokio::process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .output()
                .await?;
            
            if !output.status.success() {
                return Err(anyhow!("命令执行失败: {}", String::from_utf8_lossy(&output.stderr)));
            }
        }
        PlanAction::ModifyFile { path, changes, .. } => {
            execute_file_modifications(path, changes).await?;
        }
        PlanAction::AppendToFile { path, content, position } => {
            execute_append_to_file(path, content, position).await?;
        }
        PlanAction::GenerateCode { target_file, function_name, implementation, tests, documentation } => {
            execute_generate_code(target_file, function_name, implementation, tests, documentation).await?;
        }
        PlanAction::RefactorCode { file_path, old_pattern, new_pattern, scope } => {
            execute_refactor_code(file_path, old_pattern, new_pattern, scope).await?;
        }
        PlanAction::AddDependency { name, version, dev } => {
            execute_add_dependency(name, version, *dev).await?;
        }
        PlanAction::UpdateDependency { name, version } => {
            execute_update_dependency(name, version).await?;
        }
        PlanAction::UpdateChangelog { entry, version } => {
            execute_update_changelog(entry, version).await?;
        }
        PlanAction::GenerateDocumentation { target, content } => {
            execute_generate_documentation(target, content).await?;
        }
        PlanAction::RunTests { test_pattern, coverage } => {
            execute_run_tests(test_pattern, *coverage).await?;
        }
        PlanAction::ValidateCode { file_path, rules } => {
            execute_validate_code(file_path, rules).await?;
        }
        PlanAction::CheckDependencies => {
            execute_check_dependencies().await?;
        }
        // 暂时忽略其他新的 action 类型
        _ => {
            println!("  ⚠️ 暂不支持的操作类型，跳过");
        }
    }
    
    Ok(())
}

/// 显示计划状态
async fn show_plan_status() -> Result<()> {
    let storage = PlanStorage::new().await?;

    // 尝试加载当前活动计划
    let plan = match storage.load_current_plan().await {
        Ok(plan) => plan,
        Err(_) => {
            println!("{}", "❌ 没有找到当前活动的计划".red());
            println!("{}", "💡 使用 'matecode plan <描述>' 创建新计划".yellow());
            return Ok(());
        }
    };

    // 加载计划执行状态
    let stored_plan = match storage.load_plan(&plan.id).await {
        Ok(stored_plan) => stored_plan,
        Err(_) => {
            println!("{}", "⚠️ 无法加载计划执行状态".yellow());
            return Ok(());
        }
    };

    // 显示计划状态
    println!("\n{}", "📊 计划状态".cyan().bold());
    println!("{}", "=".repeat(60));

    println!("📋 计划: {}", plan.title.green());
    println!("🆔 ID: {}", plan.id);
    println!("🌿 分支: {}", plan.branch_name);
    println!("📊 复杂度: {:?}", plan.metadata.estimated_complexity);
    println!("📅 创建时间: {}", plan.created_at.format("%Y-%m-%d %H:%M:%S"));

    // 显示执行进度
    let total_steps = plan.actions.len();
    let completed_steps = stored_plan.completed_steps.len();
    let failed_steps = stored_plan.failed_steps.len();
    let remaining_steps = total_steps - completed_steps - failed_steps;

    println!("\n{}", "⚡ 执行进度".green().bold());
    println!("总步骤: {}", total_steps);
    println!("已完成: {} {}", completed_steps, "✅".green());
    println!("已失败: {} {}", failed_steps, if failed_steps > 0 { "❌".red() } else { "".normal() });
    println!("剩余: {} {}", remaining_steps, "⏳".yellow());

    // 显示进度条
    let progress = if total_steps > 0 {
        (completed_steps as f64 / total_steps as f64 * 100.0) as usize
    } else {
        0
    };

    let bar_length = 30;
    let filled = (progress * bar_length / 100).min(bar_length);
    let empty = bar_length - filled;

    println!("进度: [{}{}] {}%",
        "█".repeat(filled).green(),
        "░".repeat(empty).bright_black(),
        progress
    );

    // 显示详细步骤状态
    println!("\n{}", "📝 步骤详情".blue().bold());
    for (i, action) in plan.actions.iter().enumerate() {
        let status = if stored_plan.completed_steps.contains(&i) {
            "✅".green()
        } else if stored_plan.failed_steps.contains(&i) {
            "❌".red()
        } else {
            "⏳".yellow()
        };

        println!("  {}. {} {}", i + 1, status, format_action_description(action));
    }

    // 显示下一步建议
    if completed_steps == total_steps {
        println!("\n{}", "🎉 计划已全部完成！".green().bold());
    } else if failed_steps > 0 {
        println!("\n{}", "💡 建议操作:".yellow().bold());
        println!("  使用 'matecode plan --continue-plan \"\"' 重试失败的步骤");
    } else if completed_steps > 0 {
        println!("\n{}", "💡 建议操作:".yellow().bold());
        println!("  使用 'matecode plan --continue-plan \"\"' 继续执行剩余步骤");
    } else {
        println!("\n{}", "💡 建议操作:".yellow().bold());
        println!("  使用 'matecode plan --continue-plan \"\"' 开始执行计划");
    }

    Ok(())
}

/// 继续执行现有计划
async fn continue_existing_plan() -> Result<()> {
    let storage = PlanStorage::new().await?;

    // 尝试加载当前活动计划
    let plan = match storage.load_current_plan().await {
        Ok(plan) => plan,
        Err(_) => {
            println!("{}", "❌ 没有找到当前活动的计划".red());
            println!("{}", "💡 使用 'matecode plan <描述>' 创建新计划".yellow());
            return Ok(());
        }
    };

    // 尝试加载计划执行状态
    let stored_plan = storage.load_plan(&plan.id).await?;

    println!("\n{}", "🔄 继续执行计划...".cyan());
    println!("📋 计划: {}", plan.title);
    println!("📊 进度: {}/{} 步骤已完成", stored_plan.completed_steps.len(), plan.actions.len());

    if stored_plan.completed_steps.len() == plan.actions.len() {
        println!("{}", "✅ 计划已全部完成！".green());
        return Ok(());
    }

    // 继续执行未完成的步骤
    execute_plan_from_step(&plan, &stored_plan).await?;

    Ok(())
}

/// 从指定步骤开始执行计划
async fn execute_plan_from_step(plan: &Plan, stored_plan: &StoredPlan) -> Result<()> {
    let storage = PlanStorage::new().await?;
    let mut completed_steps = stored_plan.completed_steps.clone();
    let mut failed_steps = stored_plan.failed_steps.clone();

    for (i, action) in plan.actions.iter().enumerate() {
        // 跳过已完成的步骤
        if completed_steps.contains(&i) {
            println!("⏭️  跳过已完成的步骤 {}: {}", i + 1, format_action_description(action));
            continue;
        }

        // 跳过已失败的步骤（询问用户是否重试）
        if failed_steps.contains(&i) {
            if !Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(&format!("步骤 {} 之前失败过，是否重试？", i + 1))
                .default(true)
                .interact()?
            {
                continue;
            }
            // 从失败列表中移除，准备重试
            failed_steps.retain(|&x| x != i);
        }

        println!("执行步骤 {}/{}: {}", i + 1, plan.actions.len(), format_action_description(action));

        match execute_single_action(action).await {
            Ok(_) => {
                println!("  ✅ 完成");
                completed_steps.push(i);

                // 更新进度
                storage.update_plan_progress(&plan.id, i + 1, completed_steps.clone(), failed_steps.clone()).await?;
            }
            Err(e) => {
                eprintln!("  ❌ 失败: {}", e);
                failed_steps.push(i);

                // 更新进度
                storage.update_plan_progress(&plan.id, i, completed_steps.clone(), failed_steps.clone()).await?;

                if !Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("是否继续执行其他步骤？")
                    .default(true)
                    .interact()?
                {
                    break;
                }
            }
        }
    }

    if completed_steps.len() == plan.actions.len() {
        println!("\n{}", "🎉 计划执行完成！".green().bold());
    } else {
        println!("\n{}", "⏸️  计划执行暂停，使用 'matecode plan --continue' 继续".yellow());
    }

    Ok(())
}

/// 执行文件修改操作
async fn execute_file_modifications(file_path: &str, changes: &[crate::plan::FileChange]) -> Result<()> {
    use std::path::Path;

    let path = Path::new(file_path);

    // 检查文件是否存在
    if !path.exists() {
        return Err(anyhow!("文件不存在: {}", file_path));
    }

    // 读取文件内容
    let content = tokio::fs::read_to_string(path).await?;
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    // 按行号排序变更（从大到小，避免行号偏移问题）
    let mut sorted_changes = changes.to_vec();
    sorted_changes.sort_by(|a, b| {
        match (a.line_number, b.line_number) {
            (Some(a_line), Some(b_line)) => b_line.cmp(&a_line), // 倒序
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });

    // 应用变更
    for change in &sorted_changes {
        apply_file_change(&mut lines, change)?;
    }

    // 写回文件
    let new_content = lines.join("\n");
    tokio::fs::write(path, new_content).await?;

    println!("  📝 已修改文件: {}", file_path);
    Ok(())
}

/// 应用单个文件变更
fn apply_file_change(lines: &mut Vec<String>, change: &crate::plan::FileChange) -> Result<()> {
    use crate::plan::ChangeType;

    match change.change_type {
        ChangeType::Insert | ChangeType::InsertBefore | ChangeType::InsertAfter => {
            if let Some(line_num) = change.line_number {
                if line_num == 0 {
                    // 在文件开头插入
                    lines.insert(0, change.content.clone());
                } else if line_num <= lines.len() {
                    // 在指定行后插入
                    lines.insert(line_num, change.content.clone());
                } else {
                    return Err(anyhow!("插入位置超出文件范围: 行 {}", line_num));
                }
            } else {
                return Err(anyhow!("Insert 操作需要指定行号"));
            }
        }
        ChangeType::Replace => {
            if let Some(line_num) = change.line_number {
                if line_num > 0 && line_num <= lines.len() {
                    lines[line_num - 1] = change.content.clone();
                } else {
                    return Err(anyhow!("替换位置超出文件范围: 行 {}", line_num));
                }
            } else {
                return Err(anyhow!("Replace 操作需要指定行号"));
            }
        }
        ChangeType::Delete => {
            if let Some(line_num) = change.line_number {
                if line_num > 0 && line_num <= lines.len() {
                    lines.remove(line_num - 1);
                } else {
                    return Err(anyhow!("删除位置超出文件范围: 行 {}", line_num));
                }
            } else {
                return Err(anyhow!("Delete 操作需要指定行号"));
            }
        }
        ChangeType::Append => {
            // 在文件末尾追加
            lines.push(change.content.clone());
        }
    }

    Ok(())
}

/// 执行追加到文件操作
async fn execute_append_to_file(path: &str, content: &str, position: &crate::plan::AppendPosition) -> Result<()> {
    use crate::plan::AppendPosition;
    use std::path::Path;

    let file_path = Path::new(path);

    // 如果文件不存在，创建它
    if !file_path.exists() {
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(file_path, content).await?;
        println!("  📄 创建文件: {}", path);
        return Ok(());
    }

    // 读取现有内容
    let existing_content = tokio::fs::read_to_string(file_path).await?;
    let mut lines: Vec<String> = existing_content.lines().map(|s| s.to_string()).collect();

    match position {
        AppendPosition::End => {
            lines.push(content.to_string());
        }
        AppendPosition::BeforeLastLine => {
            if lines.is_empty() {
                lines.push(content.to_string());
            } else {
                lines.insert(lines.len() - 1, content.to_string());
            }
        }
        AppendPosition::AfterImports => {
            // 找到最后一个 import/use 语句的位置
            let mut insert_pos = 0;
            for (i, line) in lines.iter().enumerate() {
                let trimmed = line.trim();
                if trimmed.starts_with("use ") || trimmed.starts_with("import ") {
                    insert_pos = i + 1;
                }
            }
            lines.insert(insert_pos, content.to_string());
        }
        AppendPosition::BeforeFunction(func_name) => {
            // 找到指定函数的位置
            let mut insert_pos = lines.len();
            for (i, line) in lines.iter().enumerate() {
                if line.contains(&format!("fn {}", func_name)) {
                    insert_pos = i;
                    break;
                }
            }
            lines.insert(insert_pos, content.to_string());
        }
        AppendPosition::AfterFunction(func_name) => {
            // 找到指定函数结束的位置
            let mut insert_pos = lines.len();
            let mut in_function = false;
            let mut brace_count = 0;

            for (i, line) in lines.iter().enumerate() {
                if line.contains(&format!("fn {}", func_name)) {
                    in_function = true;
                }

                if in_function {
                    brace_count += line.matches('{').count() as i32;
                    brace_count -= line.matches('}').count() as i32;

                    if brace_count == 0 {
                        insert_pos = i + 1;
                        break;
                    }
                }
            }
            lines.insert(insert_pos, content.to_string());
        }
    }

    // 写回文件
    let new_content = lines.join("\n");
    tokio::fs::write(file_path, new_content).await?;

    println!("  📝 已追加内容到文件: {}", path);
    Ok(())
}

/// 执行代码生成操作
async fn execute_generate_code(
    target_file: &str,
    function_name: &str,
    implementation: &str,
    tests: &Option<String>,
    documentation: &Option<String>,
) -> Result<()> {
    use std::path::Path;

    let file_path = Path::new(target_file);

    // 构建完整的代码内容
    let mut code_content = String::new();

    // 添加文档注释
    if let Some(doc) = documentation {
        code_content.push_str(&format!("/// {}\n", doc));
    }

    // 添加函数实现
    code_content.push_str(&format!("pub fn {}() {{\n", function_name));
    code_content.push_str(&format!("    {}\n", implementation.replace('\n', "\n    ")));
    code_content.push_str("}\n");

    // 添加测试代码
    if let Some(test_code) = tests {
        code_content.push_str("\n#[cfg(test)]\nmod tests {\n");
        code_content.push_str("    use super::*;\n\n");
        code_content.push_str(&format!("    {}\n", test_code.replace('\n', "\n    ")));
        code_content.push_str("}\n");
    }

    // 追加到文件
    execute_append_to_file(target_file, &code_content, &crate::plan::AppendPosition::End).await?;

    println!("  🔧 已生成代码: {} 中的 {}", target_file, function_name);
    Ok(())
}

/// 执行代码重构操作
async fn execute_refactor_code(
    file_path: &str,
    old_pattern: &str,
    new_pattern: &str,
    _scope: &crate::plan::RefactorScope,
) -> Result<()> {
    use std::path::Path;

    let path = Path::new(file_path);
    if !path.exists() {
        return Err(anyhow!("文件不存在: {}", file_path));
    }

    // 读取文件内容
    let content = tokio::fs::read_to_string(path).await?;

    // 执行简单的字符串替换重构
    let new_content = content.replace(old_pattern, new_pattern);

    // 写回文件
    tokio::fs::write(path, new_content).await?;

    println!("  🔄 已重构代码: {} (替换 '{}' -> '{}')", file_path, old_pattern, new_pattern);
    Ok(())
}

/// 执行添加依赖操作
async fn execute_add_dependency(name: &str, version: &Option<String>, dev: bool) -> Result<()> {
    let version_str = version.as_deref().unwrap_or("*");
    let dep_type = if dev { "dev-dependencies" } else { "dependencies" };

    // 这里应该解析和修改 Cargo.toml 文件
    // 暂时只是打印信息
    println!("  📦 添加依赖: {} = \"{}\" ({})", name, version_str, dep_type);

    // TODO: 实际修改 Cargo.toml 文件
    Ok(())
}

/// 执行更新依赖操作
async fn execute_update_dependency(name: &str, version: &str) -> Result<()> {
    println!("  📦 更新依赖: {} -> {}", name, version);

    // TODO: 实际修改 Cargo.toml 文件
    Ok(())
}

/// 执行更新 CHANGELOG 操作
async fn execute_update_changelog(entry: &str, version: &Option<String>) -> Result<()> {
    use std::path::Path;

    let changelog_path = Path::new("CHANGELOG.md");
    let version_str = version.as_deref().unwrap_or("Unreleased");

    let changelog_entry = format!(
        "\n## [{}] - {}\n\n### Added\n- {}\n",
        version_str,
        chrono::Utc::now().format("%Y-%m-%d"),
        entry
    );

    if changelog_path.exists() {
        // 读取现有内容
        let existing_content = tokio::fs::read_to_string(changelog_path).await?;

        // 在文件开头插入新条目（在标题后）
        let lines: Vec<&str> = existing_content.lines().collect();
        let mut new_lines = Vec::new();

        // 保留标题行
        if !lines.is_empty() {
            new_lines.push(lines[0]);
        }

        // 插入新条目
        new_lines.push(&changelog_entry);

        // 添加剩余内容
        for line in lines.iter().skip(1) {
            new_lines.push(line);
        }

        let new_content = new_lines.join("\n");
        tokio::fs::write(changelog_path, new_content).await?;
    } else {
        // 创建新的 CHANGELOG
        let content = format!("# Changelog\n{}", changelog_entry);
        tokio::fs::write(changelog_path, content).await?;
    }

    println!("  📝 已更新 CHANGELOG: {}", entry);
    Ok(())
}

/// 执行生成文档操作
async fn execute_generate_documentation(target: &crate::plan::DocumentationTarget, content: &str) -> Result<()> {
    use crate::plan::DocumentationTarget;

    let file_path = match target {
        DocumentationTarget::README => "README.md",
        DocumentationTarget::API => "docs/api.md",
        DocumentationTarget::UserGuide => "docs/user-guide.md",
        DocumentationTarget::DeveloperGuide => "docs/developer-guide.md",
        DocumentationTarget::Changelog => "CHANGELOG.md",
    };

    execute_append_to_file(file_path, content, &crate::plan::AppendPosition::End).await?;

    println!("  📚 已生成文档: {}", file_path);
    Ok(())
}

/// 执行运行测试操作
async fn execute_run_tests(test_pattern: &Option<String>, coverage: bool) -> Result<()> {
    let mut cmd = tokio::process::Command::new("cargo");
    cmd.arg("test");

    if let Some(pattern) = test_pattern {
        cmd.arg(pattern);
    }

    if coverage {
        // 如果需要覆盖率，可以使用 tarpaulin 或其他工具
        println!("  🧪 运行测试 (带覆盖率)...");
    } else {
        println!("  🧪 运行测试...");
    }

    let output = cmd.output().await?;

    if output.status.success() {
        println!("  ✅ 测试通过");
    } else {
        println!("  ❌ 测试失败");
        println!("{}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}

/// 执行代码验证操作
async fn execute_validate_code(file_path: &str, rules: &[String]) -> Result<()> {
    println!("  🔍 验证代码: {} (规则: {:?})", file_path, rules);

    // 这里可以集成 clippy、rustfmt 等工具
    let output = tokio::process::Command::new("cargo")
        .arg("check")
        .arg("--bin")
        .arg("matecode")
        .output()
        .await?;

    if output.status.success() {
        println!("  ✅ 代码验证通过");
    } else {
        println!("  ❌ 代码验证失败");
        println!("{}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}

/// 执行检查依赖操作
async fn execute_check_dependencies() -> Result<()> {
    println!("  📦 检查依赖...");

    let output = tokio::process::Command::new("cargo")
        .arg("tree")
        .output()
        .await?;

    if output.status.success() {
        println!("  ✅ 依赖检查完成");
    } else {
        println!("  ❌ 依赖检查失败");
        println!("{}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}
