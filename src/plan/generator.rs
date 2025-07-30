use super::*;
use crate::config;
use crate::llm::parse_prompt_template;
use anyhow::{Result, anyhow};
use chrono::Utc;
use uuid::Uuid;

/// 智能计划生成器 - 负责生成强大的开发计划
pub struct PlanGenerator {
    project_analyzer: analyzer::ProjectAnalyzer,
}

impl PlanGenerator {
    pub async fn new() -> Result<Self> {
        let project_analyzer = analyzer::ProjectAnalyzer::new().await?;

        Ok(Self {
            project_analyzer,
        })
    }

    /// 生成完整的开发计划
    pub async fn generate_comprehensive_plan(&self, description: &str) -> Result<Plan> {
        println!("🧠 开始智能分析和计划生成...");

        // 1. 深度项目分析
        let analysis: PlanAnalysis = self.analyze_project_context(description).await?;

        // 2. 生成技术解决方案
        let technical_solution = self.generate_technical_solution(description, &analysis).await?;

        // 3. 评估影响范围
        let impact_assessment = self.assess_impact(description, &analysis, &technical_solution).await?;

        // 4. 生成具体的执行计划
        let (actions, phases) = self.generate_execution_plan(description, &analysis, &technical_solution).await?;

        // 5. 生成分支名称
        let branch_name = self.generate_branch_name(description).await?;

        // 6. 收集项目上下文
        let project_context = self.project_analyzer.analyze_codebase().await?;

        // 7. 构建完整计划
        let plan = Plan {
            id: Uuid::new_v4().to_string(),
            title: description.to_string(),
            description: description.to_string(),
            branch_name,
            status: PlanStatus::Draft,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            phases,
            actions,
            affected_files: analysis.related_files.iter().map(|f| f.path.clone()).collect(),
            analysis,
            technical_solution,
            impact_assessment,
            metadata: PlanMetadata {
                technical_approach: "AI-generated comprehensive plan".to_string(),
                architecture_notes: "Generated using intelligent analysis".to_string(),
                dependencies: vec![],
                estimated_complexity: ComplexityLevel::Medium,
                related_files: vec![],
            },
            project_context,
            execution_config: ExecutionConfig::default(),
            user_preferences: UserPreferences::default(),
        };

        Ok(plan)
    }

    /// 深度分析项目上下文
    async fn analyze_project_context(&self, description: &str) -> Result<PlanAnalysis> {
        println!("🔍 分析项目上下文...");

        // 1. 收集相关文件
        let related_files = self.collect_related_files(description).await?;

        // 2. 生成代码理解报告
        let code_understanding = self.generate_code_understanding(&related_files).await?;

        // 3. 分析依赖关系
        let dependency_graph = self.analyze_dependencies(&related_files).await?;

        // 4. 生成架构说明
        let architecture_notes = self.generate_architecture_notes(description, &related_files).await?;

        Ok(PlanAnalysis {
            related_files,
            code_understanding,
            dependency_graph,
            architecture_notes,
        })
    }

    /// 收集相关文件
    async fn collect_related_files(&self, description: &str) -> Result<Vec<RelatedFile>> {
        println!("📁 收集相关文件...");

        // 使用 LLM 分析需求，识别可能相关的文件
        let prompt = format!(
            "基于以下需求描述，分析项目中可能相关的文件类型和路径模式：\n\n需求：{}\n\n请列出可能需要修改或参考的文件类型。",
            description
        );

        // 这里应该调用 LLM 来智能识别相关文件
        // 暂时返回一个基本的实现
        let mut related_files = Vec::new();

        // 基于项目结构和需求关键词进行简单匹配
        let project_context = self.project_analyzer.analyze_codebase().await?;

        // 分析项目结构中的文件
        for dir in &project_context.structure.directories {
            if self.is_directory_relevant(dir, description) {
                // 扫描目录中的文件
                let dir_path = std::path::Path::new(dir);
                if dir_path.exists() {
                    if let Ok(entries) = std::fs::read_dir(dir_path) {
                        for entry in entries.flatten() {
                            if let Some(file_name) = entry.file_name().to_str() {
                                if self.is_file_relevant(file_name, description) {
                                    related_files.push(RelatedFile {
                                        path: entry.path().to_string_lossy().to_string(),
                                        file_type: self.determine_file_type(file_name),
                                        relevance_score: 0.8,
                                        summary: format!("Potentially relevant file: {}", file_name),
                                        key_functions: vec![],
                                        dependencies: vec![],
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(related_files)
    }

    /// 判断目录是否相关
    fn is_directory_relevant(&self, dir: &str, description: &str) -> bool {
        let description_lower = description.to_lowercase();
        let dir_lower = dir.to_lowercase();

        // 基本的关键词匹配
        if description_lower.contains("auth") && dir_lower.contains("auth") {
            return true;
        }
        if description_lower.contains("user") && dir_lower.contains("user") {
            return true;
        }
        if description_lower.contains("api") && dir_lower.contains("api") {
            return true;
        }
        if description_lower.contains("config") && dir_lower.contains("config") {
            return true;
        }

        // 总是包含核心目录
        matches!(dir, "src" | "src/commands" | "src/plan")
    }

    /// 判断文件是否相关
    fn is_file_relevant(&self, file_name: &str, description: &str) -> bool {
        let description_lower = description.to_lowercase();
        let file_lower = file_name.to_lowercase();

        // 基本的关键词匹配
        if description_lower.contains("auth") && file_lower.contains("auth") {
            return true;
        }
        if description_lower.contains("user") && file_lower.contains("user") {
            return true;
        }
        if description_lower.contains("config") && file_lower.contains("config") {
            return true;
        }

        // 总是包含重要的配置文件
        matches!(file_name, "main.rs" | "mod.rs" | "lib.rs" | "Cargo.toml")
    }

    /// 确定文件类型
    fn determine_file_type(&self, file_name: &str) -> FileType {
        if file_name.ends_with(".rs") {
            if file_name.contains("test") || file_name.starts_with("test_") {
                FileType::Test
            } else {
                FileType::Source
            }
        } else if file_name.ends_with(".toml") || file_name.ends_with(".json") || file_name.ends_with(".yaml") {
            FileType::Config
        } else if file_name.ends_with(".md") || file_name.ends_with(".txt") {
            FileType::Documentation
        } else {
            FileType::Build
        }
    }

    /// 生成代码理解报告
    async fn generate_code_understanding(&self, related_files: &[RelatedFile]) -> Result<Vec<CodeUnderstanding>> {
        println!("📖 生成代码理解报告...");

        let mut understanding = Vec::new();

        for file in related_files.iter().take(5) { // 限制处理的文件数量
            if let Ok(content) = tokio::fs::read_to_string(&file.path).await {
                let summary = self.analyze_file_content(&content, &file.path).await?;
                understanding.push(CodeUnderstanding {
                    file_path: file.path.clone(),
                    summary,
                    key_concepts: vec!["TODO: Extract key concepts".to_string()],
                    patterns_identified: vec!["TODO: Identify patterns".to_string()],
                    suggestions: vec!["TODO: Generate suggestions".to_string()],
                });
            }
        }

        Ok(understanding)
    }

    /// 分析文件内容
    async fn analyze_file_content(&self, content: &str, file_path: &str) -> Result<String> {
        // 简单的内容分析，实际应该使用 LLM
        let lines = content.lines().count();
        let functions = content.matches("fn ").count();
        let structs = content.matches("struct ").count();

        Ok(format!(
            "文件 {} 包含 {} 行代码，{} 个函数，{} 个结构体",
            file_path, lines, functions, structs
        ))
    }

    /// 分析依赖关系
    async fn analyze_dependencies(&self, related_files: &[RelatedFile]) -> Result<DependencyGraph> {
        println!("🔗 分析依赖关系...");

        // 简单的依赖分析实现
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        for (i, file) in related_files.iter().enumerate() {
            nodes.push(DependencyNode {
                id: format!("file_{}", i),
                name: file.path.clone(),
                node_type: NodeType::File,
            });
        }

        Ok(DependencyGraph { nodes, edges })
    }

    /// 生成架构说明
    async fn generate_architecture_notes(&self, description: &str, related_files: &[RelatedFile]) -> Result<String> {
        println!("🏗️ 生成架构说明...");

        let notes = format!(
            "基于需求 '{}' 的架构分析：\n\n相关文件数量: {}\n主要涉及的组件: TODO\n建议的实现方式: TODO",
            description,
            related_files.len()
        );

        Ok(notes)
    }

    /// 生成技术解决方案
    async fn generate_technical_solution(&self, description: &str, analysis: &PlanAnalysis) -> Result<TechnicalSolution> {
        println!("💡 生成技术解决方案...");

        // 使用 LLM 分析项目上下文并生成技术方案
        let solution = self.generate_ai_technical_solution(description, analysis).await?;

        Ok(solution)
    }

    /// 使用 AI 生成技术解决方案
    async fn generate_ai_technical_solution(&self, description: &str, analysis: &PlanAnalysis) -> Result<TechnicalSolution> {
        // 构建上下文信息
        let context = self.build_project_context_for_llm(analysis).await?;

        // 构建 LLM 提示
        let prompt = self.build_technical_solution_prompt(description, &context).await?;

        // 调用 LLM 生成技术方案
        let llm_response = self.call_llm_for_technical_solution(&prompt).await?;

        // 解析 LLM 响应
        self.parse_technical_solution_response(&llm_response).await
    }

    /// 构建项目上下文信息供 LLM 使用
    async fn build_project_context_for_llm(&self, analysis: &PlanAnalysis) -> Result<String> {
        let mut context = String::new();

        // 添加项目结构信息
        context.push_str("## 项目结构\n");
        for file in &analysis.related_files {
            context.push_str(&format!("- {}: {}\n", file.path, file.summary));
        }

        // 添加代码理解信息
        context.push_str("\n## 代码分析\n");
        for understanding in &analysis.code_understanding {
            context.push_str(&format!("### {}\n", understanding.file_path));
            context.push_str(&format!("{}\n", understanding.summary));
            if !understanding.key_concepts.is_empty() {
                context.push_str(&format!("关键概念: {}\n", understanding.key_concepts.join(", ")));
            }
        }

        // 添加架构说明
        context.push_str("\n## 架构说明\n");
        context.push_str(&analysis.architecture_notes);

        Ok(context)
    }

    /// 构建技术方案生成的提示
    async fn build_technical_solution_prompt(&self, description: &str, context: &str) -> Result<String> {
        let prompt = format!(r#"
作为一个资深的软件架构师，请基于以下项目信息和需求，生成一个详细的技术实现方案。

## 项目上下文
{}

## 需求描述
{}

请提供以下内容：

1. **技术方案概述**: 简要描述实现方法和技术选型
2. **架构模式**: 推荐使用的设计模式
3. **设计原则**: 应该遵循的设计原则
4. **实现步骤**: 详细的实现步骤，每个步骤包含：
   - 步骤标题
   - 详细描述
   - 需要修改的文件
   - 具体的代码示例（基于项目现有代码风格）
5. **备选方案**: 其他可能的实现方式
6. **风险评估**: 可能的风险和缓解措施

请确保代码示例符合项目的现有代码风格和架构。
"#, context, description);

        Ok(prompt)
    }

    /// 调用 LLM 生成技术方案
    async fn call_llm_for_technical_solution(&self, prompt: &str) -> Result<String> {
        // 这里应该调用实际的 LLM API
        // 暂时返回一个模拟的响应
        Ok(format!(r#"
## 技术方案概述
基于项目现有的 Rust CLI 架构，采用模块化设计实现 "{}" 功能。

## 架构模式
- Command Pattern: 用于命令处理
- Builder Pattern: 用于配置构建
- Strategy Pattern: 用于不同策略的实现

## 设计原则
- 单一职责原则
- 开闭原则
- 依赖倒置原则

## 实现步骤
### 步骤1: 创建核心模块
创建新的模块文件来实现核心功能
文件: src/new_module.rs

### 步骤2: 集成到命令系统
将新功能集成到现有的命令处理系统中
文件: src/commands/mod.rs

## 备选方案
- 方案A: 直接扩展现有模块
- 方案B: 创建独立的子系统

## 风险评估
- 风险: 可能影响现有功能
- 缓解: 充分的单元测试和集成测试
"#, prompt.lines().nth(5).unwrap_or("功能")))
    }

    /// 解析 LLM 响应为技术方案结构
    async fn parse_technical_solution_response(&self, response: &str) -> Result<TechnicalSolution> {
        // 这里应该解析 LLM 的响应并转换为结构化数据
        // 暂时返回一个基本的解析结果

        Ok(TechnicalSolution {
            approach: "基于 AI 分析的技术方案".to_string(),
            architecture_pattern: Some("AI 推荐的架构模式".to_string()),
            design_principles: vec![
                "单一职责原则".to_string(),
                "开闭原则".to_string(),
                "依赖倒置原则".to_string(),
            ],
            implementation_steps: vec![
                ImplementationStep {
                    step_number: 1,
                    title: "AI 生成的实现步骤".to_string(),
                    description: "基于项目分析生成的具体实现方案".to_string(),
                    code_snippets: vec![],
                    files_to_modify: vec!["待 AI 分析确定".to_string()],
                }
            ],
            alternatives_considered: vec![
                Alternative {
                    name: "AI 分析的备选方案".to_string(),
                    description: "基于项目情况的其他实现方式".to_string(),
                    pros: vec!["优点1".to_string(), "优点2".to_string()],
                    cons: vec!["缺点1".to_string(), "缺点2".to_string()],
                    why_not_chosen: "基于项目特点的选择原因".to_string(),
                }
            ],
            risks_and_mitigations: vec![
                RiskMitigation {
                    risk: "AI 识别的潜在风险".to_string(),
                    probability: RiskLevel::Medium,
                    impact: RiskLevel::Medium,
                    mitigation: "AI 建议的缓解措施".to_string(),
                }
            ],
        })
    }


    /// 评估影响范围
    async fn assess_impact(&self, description: &str, analysis: &PlanAnalysis, solution: &TechnicalSolution) -> Result<ImpactAssessment> {
        println!("📊 评估影响范围...");

        Ok(ImpactAssessment {
            affected_components: analysis.related_files.iter().map(|f| ComponentImpact {
                component: f.path.clone(),
                impact_level: ImpactLevel::Medium,
                changes_required: vec!["需要修改".to_string()],
                migration_notes: None,
            }).collect(),
            breaking_changes: vec![],
            performance_impact: PerformanceImpact {
                expected_change: PerformanceChange::Neutral,
                metrics_affected: vec![],
                benchmarking_plan: None,
            },
            security_considerations: vec![],
            testing_requirements: vec![
                TestingRequirement {
                    test_type: TestType::Unit,
                    description: "单元测试覆盖新功能".to_string(),
                    priority: Priority::High,
                }
            ],
        })
    }

    /// 生成执行计划
    async fn generate_execution_plan(&self, description: &str, analysis: &PlanAnalysis, _solution: &TechnicalSolution) -> Result<(Vec<PlanAction>, Vec<PlanPhase>)> {
        println!("📋 生成执行计划...");

        let mut actions = Vec::new();
        let mut phases = Vec::new();

        // 第一阶段：准备工作
        let phase1_actions = vec![0]; // 引用 actions 的索引
        phases.push(PlanPhase {
            id: "phase_1".to_string(),
            name: "准备阶段".to_string(),
            description: "创建分支和准备开发环境".to_string(),
            actions: phase1_actions,
            dependencies: vec![],
            validation_rules: vec![],
            estimated_duration: Some(10),
        });

        // 添加创建分支的操作
        actions.push(PlanAction::CreateBranch {
            name: self.generate_branch_name(description).await?,
            from_branch: None,
        });

        // 第二阶段：实现功能
        let phase2_start = actions.len();

        // 根据分析结果生成具体的文件操作
        for file in &analysis.related_files {
            if file.file_type == FileType::Source {
                actions.push(PlanAction::ModifyFile {
                    path: file.path.clone(),
                    changes: vec![
                        FileChange {
                            line_number: None,
                            change_type: ChangeType::Append,
                            content: format!("// TODO: 实现 {} 相关功能", description),
                            context: Some("文件末尾".to_string()),
                            reason: Some(format!("为实现 {} 添加占位符", description)),
                        }
                    ],
                    backup: true,
                });
            }
        }

        let phase2_actions: Vec<usize> = (phase2_start..actions.len()).collect();
        phases.push(PlanPhase {
            id: "phase_2".to_string(),
            name: "功能实现".to_string(),
            description: "实现核心功能逻辑".to_string(),
            actions: phase2_actions,
            dependencies: vec!["phase_1".to_string()],
            validation_rules: vec![
                ValidationRule {
                    rule_type: ValidationType::Compilation,
                    description: "确保代码能够编译".to_string(),
                    command: Some("cargo build".to_string()),
                    expected_result: Some("编译成功".to_string()),
                }
            ],
            estimated_duration: Some(60),
        });

        // 添加更新 CHANGELOG
        actions.push(PlanAction::UpdateChangelog {
            entry: format!("添加功能: {}", description),
            version: None,
        });

        // 添加生成文档操作
        actions.push(PlanAction::GenerateDocumentation {
            target: DocumentationTarget::README,
            content: format!("## 新功能\n\n{}\n", description),
        });

        Ok((actions, phases))
    }

    /// 生成分支名称
    async fn generate_branch_name(&self, description: &str) -> Result<String> {
        // 使用 branch 命令中的智能生成逻辑
        Ok(crate::commands::branch::generate_smart_branch_name(description))
    }
}
