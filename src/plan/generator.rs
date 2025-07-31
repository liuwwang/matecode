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

    /// 生成简单的开发计划（模板模式，不使用 LLM）
    pub async fn generate_simple_plan(&self, description: &str) -> Result<Plan> {
        println!("📝 使用简单模板生成计划...");

        // 生成分支名称
        let branch_name = crate::commands::branch::generate_smart_branch_name(description);

        // 创建简单的计划
        let plan = Plan {
            id: Uuid::new_v4().to_string(),
            title: description.to_string(),
            description: description.to_string(),
            branch_name,
            status: PlanStatus::Draft,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            phases: self.generate_simple_phases(description),
            actions: self.generate_simple_actions(description),
            affected_files: vec![], // 简单模式不分析文件
            analysis: self.create_simple_analysis(description),
            technical_solution: self.create_simple_solution(description),
            impact_assessment: self.create_simple_impact(),
            metadata: self.create_simple_metadata(),
            project_context: self.create_simple_context().await?,
            execution_config: ExecutionConfig::default(),
            user_preferences: UserPreferences::default(),
        };

        Ok(plan)
    }

    /// 生成完整的开发计划 (重构版：专注于高层次规划)
    pub async fn generate_comprehensive_plan(&self, description: &str) -> Result<Plan> {
        println!("🧠 开始智能分析和计划生成...");

        // 1. 深度项目分析 - 理解现有代码结构
        let analysis: PlanAnalysis = self.analyze_project_context(description).await?;

        // 2. 需求理解和分解 - AI 理解用户真正想要什么
        let requirement_analysis = self.analyze_requirement(description, &analysis).await?;

        // 3. 生成高层次技术方案 - 不包含具体代码
        let technical_solution = self.generate_high_level_solution(description, &analysis, &requirement_analysis).await?;

        // 4. 评估影响范围 - 分析会影响哪些文件和组件
        let impact_assessment = self.assess_impact(description, &analysis, &technical_solution).await?;

        // 5. 生成抽象执行计划 - 高层次步骤，不包含具体代码
        let (actions, phases) = self.generate_abstract_execution_plan(description, &analysis, &technical_solution, &requirement_analysis).await?;

        // 6. 生成分支名称
        let branch_name = self.generate_branch_name(description).await?;

        // 7. 收集项目上下文
        let project_context = self.project_analyzer.analyze_codebase().await?;

        // 8. 构建完整计划
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
            analysis: analysis.clone(),
            technical_solution,
            impact_assessment,
            metadata: PlanMetadata {
                technical_approach: requirement_analysis.approach,
                architecture_notes: requirement_analysis.architecture_notes,
                dependencies: requirement_analysis.dependencies,
                estimated_complexity: requirement_analysis.complexity,
                related_files: analysis.related_files.iter().map(|f| f.path.clone()).collect(),
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

        // 简化版：直接收集核心文件，不使用复杂的相关性判断
        // 不再需要完整的项目上下文分析

        // 收集核心项目文件（主要是 src 目录下的文件）
        let core_dirs = ["src", "src/commands", "src/plan"];

        for &dir in &core_dirs {
            let dir_path = std::path::Path::new(dir);
            if dir_path.exists() {
                if let Ok(entries) = std::fs::read_dir(dir_path) {
                    for entry in entries.flatten() {
                        if let Some(file_name) = entry.file_name().to_str() {
                            // 只收集 Rust 源文件和重要配置文件
                            if file_name.ends_with(".rs") || matches!(file_name, "Cargo.toml" | "Cargo.lock") {
                                related_files.push(RelatedFile {
                                    path: entry.path().to_string_lossy().to_string(),
                                    file_type: self.determine_file_type(file_name),
                                    relevance_score: 0.7, // 统一的相关性分数
                                    summary: format!("Core project file: {}", file_name),
                                    key_functions: vec![],
                                    dependencies: vec![],
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(related_files)
    }

    // 移除了复杂的相关性判断函数 - 现在使用简单直接的文件收集方式

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

    /// 分析用户需求 - AI 深度理解用户意图
    async fn analyze_requirement(&self, description: &str, analysis: &PlanAnalysis) -> Result<RequirementAnalysis> {
        println!("🎯 分析用户需求和意图...");

        // 这里应该调用 LLM 来深度分析用户需求
        // 暂时返回基于规则的分析结果

        let intent = self.analyze_user_intent(description).await?;
        let scope = self.analyze_requirement_scope(description, analysis).await?;
        let key_components = self.identify_key_components(description, analysis).await?;

        Ok(RequirementAnalysis {
            intent,
            scope,
            approach: format!("基于 {} 的实现方案", description),
            architecture_notes: format!("针对 {} 的架构设计说明", description),
            dependencies: vec![], // 将在后续分析中填充
            complexity: self.estimate_complexity(description, &key_components).await?,
            key_components,
            constraints: vec!["保持向后兼容".to_string(), "遵循项目编码规范".to_string()],
        })
    }

    /// 分析用户意图
    async fn analyze_user_intent(&self, description: &str) -> Result<UserIntent> {
        let desc_lower = description.to_lowercase();

        let primary_goal = if desc_lower.contains("添加") || desc_lower.contains("新增") || desc_lower.contains("创建") {
            format!("添加新功能: {}", description)
        } else if desc_lower.contains("修复") || desc_lower.contains("解决") || desc_lower.contains("bug") {
            format!("修复问题: {}", description)
        } else if desc_lower.contains("优化") || desc_lower.contains("改进") || desc_lower.contains("重构") {
            format!("优化改进: {}", description)
        } else {
            format!("实现功能: {}", description)
        };

        let urgency = if desc_lower.contains("紧急") || desc_lower.contains("critical") {
            UrgencyLevel::Critical
        } else if desc_lower.contains("重要") || desc_lower.contains("high") {
            UrgencyLevel::High
        } else {
            UrgencyLevel::Medium
        };

        Ok(UserIntent {
            primary_goal,
            secondary_goals: vec!["确保代码质量".to_string(), "添加适当的测试".to_string()],
            user_type: UserType::Developer,
            urgency,
        })
    }

    /// 分析需求范围
    async fn analyze_requirement_scope(&self, description: &str, analysis: &PlanAnalysis) -> Result<RequirementScope> {
        let desc_lower = description.to_lowercase();

        let feature_type = if desc_lower.contains("修复") || desc_lower.contains("bug") {
            FeatureType::BugFix
        } else if desc_lower.contains("重构") || desc_lower.contains("优化") {
            FeatureType::Refactoring
        } else if desc_lower.contains("配置") || desc_lower.contains("config") {
            FeatureType::Configuration
        } else if desc_lower.contains("测试") || desc_lower.contains("test") {
            FeatureType::Testing
        } else if desc_lower.contains("文档") || desc_lower.contains("doc") {
            FeatureType::Documentation
        } else {
            FeatureType::NewFeature
        };

        let affected_layers = vec![
            ArchitectureLayer::Business, // 大多数功能都会影响业务逻辑层
        ];

        Ok(RequirementScope {
            feature_type,
            affected_layers,
            integration_points: analysis.related_files.iter().map(|f| f.path.clone()).collect(),
            external_dependencies: vec![],
        })
    }

    /// 识别关键组件
    async fn identify_key_components(&self, description: &str, analysis: &PlanAnalysis) -> Result<Vec<ComponentRequirement>> {
        let mut components = Vec::new();

        // 基于需求描述和项目分析识别需要的组件
        if description.contains("认证") || description.contains("登录") || description.contains("auth") {
            components.push(ComponentRequirement {
                name: "AuthenticationService".to_string(),
                purpose: "处理用户认证逻辑".to_string(),
                interfaces: vec!["login".to_string(), "logout".to_string(), "verify_token".to_string()],
                dependencies: vec!["UserRepository".to_string(), "TokenService".to_string()],
                estimated_effort: EffortLevel::Medium,
            });
        }

        if description.contains("配置") || description.contains("config") {
            components.push(ComponentRequirement {
                name: "ConfigurationManager".to_string(),
                purpose: "管理应用程序配置".to_string(),
                interfaces: vec!["load_config".to_string(), "update_config".to_string()],
                dependencies: vec!["FileSystem".to_string()],
                estimated_effort: EffortLevel::Small,
            });
        }

        // 如果没有识别到特定组件，添加一个通用组件
        if components.is_empty() {
            components.push(ComponentRequirement {
                name: "FeatureImplementation".to_string(),
                purpose: format!("实现 {} 功能", description),
                interfaces: vec!["main_function".to_string()],
                dependencies: analysis.related_files.iter().take(3).map(|f| f.path.clone()).collect(),
                estimated_effort: EffortLevel::Medium,
            });
        }

        Ok(components)
    }

    /// 估算复杂度
    async fn estimate_complexity(&self, description: &str, components: &[ComponentRequirement]) -> Result<ComplexityLevel> {
        let desc_lower = description.to_lowercase();

        // 基于关键词和组件数量估算复杂度
        let keyword_complexity = if desc_lower.contains("认证") || desc_lower.contains("安全") || desc_lower.contains("数据库") {
            2 // 高复杂度关键词
        } else if desc_lower.contains("api") || desc_lower.contains("接口") || desc_lower.contains("集成") {
            1 // 中等复杂度关键词
        } else {
            0 // 低复杂度
        };

        let component_complexity = components.len();
        let total_complexity = keyword_complexity + component_complexity;

        Ok(match total_complexity {
            0..=1 => ComplexityLevel::Low,
            2..=3 => ComplexityLevel::Medium,
            4..=5 => ComplexityLevel::High,
            _ => ComplexityLevel::VeryHigh,
        })
    }

    /// 生成高层次解决方案 - 不包含具体代码
    async fn generate_high_level_solution(&self, description: &str, analysis: &PlanAnalysis, requirement: &RequirementAnalysis) -> Result<TechnicalSolution> {
        println!("🏗️ 生成高层次技术解决方案...");

        let approach = format!(
            "采用 {} 方案实现 {}，重点关注 {}",
            match requirement.scope.feature_type {
                FeatureType::NewFeature => "模块化设计",
                FeatureType::BugFix => "最小化影响",
                FeatureType::Refactoring => "渐进式重构",
                FeatureType::Configuration => "配置驱动",
                _ => "最佳实践",
            },
            requirement.intent.primary_goal,
            requirement.key_components.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join("、")
        );

        let implementation_steps = requirement.key_components.iter().enumerate().map(|(i, component)| {
            ImplementationStep {
                step_number: i + 1,
                title: format!("实现 {}", component.name),
                description: component.purpose.clone(),
                code_snippets: vec![], // 高层次计划不包含具体代码
                files_to_modify: component.dependencies.clone(),
            }
        }).collect();

        Ok(TechnicalSolution {
            approach,
            architecture_pattern: Some("模块化架构".to_string()),
            design_principles: vec![
                "单一职责原则".to_string(),
                "开闭原则".to_string(),
                "依赖倒置原则".to_string(),
            ],
            implementation_steps,
            alternatives_considered: vec![],
            risks_and_mitigations: vec![],
        })
    }

    /// 生成抽象执行计划 - 高层次步骤，不包含具体代码
    async fn generate_abstract_execution_plan(
        &self,
        description: &str,
        analysis: &PlanAnalysis,
        solution: &TechnicalSolution,
        requirement: &RequirementAnalysis,
    ) -> Result<(Vec<PlanAction>, Vec<PlanPhase>)> {
        println!("📋 生成抽象执行计划...");

        let mut actions = Vec::new();
        let mut phases = Vec::new();

        // 第一阶段：准备工作
        let phase1_start = actions.len();

        // 创建分支
        actions.push(PlanAction::CreateBranch {
            name: self.generate_branch_name(description).await?,
            from_branch: None,
        });

        let phase1_actions: Vec<usize> = (phase1_start..actions.len()).collect();
        phases.push(PlanPhase {
            id: "phase_1".to_string(),
            name: "准备阶段".to_string(),
            description: "创建分支和准备开发环境".to_string(),
            actions: phase1_actions,
            dependencies: vec![],
            validation_rules: vec![],
            estimated_duration: Some(5),
        });

        // 第二阶段：核心实现
        let phase2_start = actions.len();

        // 为每个关键组件创建抽象的实现步骤
        for component in &requirement.key_components {
            // 这里创建的是抽象的操作，具体代码将在执行时生成
            actions.push(PlanAction::GenerateCode {
                target_file: format!("src/{}.rs", component.name.to_lowercase()),
                function_name: component.interfaces.first().unwrap_or(&"main".to_string()).clone(),
                implementation: format!("// 待实现: {}", component.purpose),
                tests: Some(format!("// 待实现: {} 的测试", component.name)),
                documentation: Some(component.purpose.clone()),
            });
        }

        let phase2_actions: Vec<usize> = (phase2_start..actions.len()).collect();
        phases.push(PlanPhase {
            id: "phase_2".to_string(),
            name: "核心实现".to_string(),
            description: "实现主要功能组件".to_string(),
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

        // 第三阶段：完善和文档
        let phase3_start = actions.len();

        actions.push(PlanAction::UpdateChangelog {
            entry: format!("添加功能: {}", description),
            version: None,
        });

        actions.push(PlanAction::GenerateDocumentation {
            target: DocumentationTarget::README,
            content: format!("## 新功能\n\n{}\n", description),
        });

        let phase3_actions: Vec<usize> = (phase3_start..actions.len()).collect();
        phases.push(PlanPhase {
            id: "phase_3".to_string(),
            name: "完善阶段".to_string(),
            description: "更新文档和测试".to_string(),
            actions: phase3_actions,
            dependencies: vec!["phase_2".to_string()],
            validation_rules: vec![],
            estimated_duration: Some(20),
        });

        Ok((actions, phases))
    }

    /// 生成分支名称
    async fn generate_branch_name(&self, description: &str) -> Result<String> {
        // 使用 branch 命令中的智能生成逻辑
        Ok(crate::commands::branch::generate_smart_branch_name(description))
    }

    // === 简单模式的辅助方法 ===

    /// 生成简单的阶段
    fn generate_simple_phases(&self, description: &str) -> Vec<PlanPhase> {
        vec![
            PlanPhase {
                id: "phase_1".to_string(),
                name: "准备阶段".to_string(),
                description: "创建分支和基础设置".to_string(),
                actions: vec![0], // 对应第一个 action
                dependencies: vec![],
                validation_rules: vec![],
                estimated_duration: Some(5),
            },
            PlanPhase {
                id: "phase_2".to_string(),
                name: "实现阶段".to_string(),
                description: format!("实现 {}", description),
                actions: vec![1, 2], // 对应后续 actions
                dependencies: vec!["phase_1".to_string()],
                validation_rules: vec![
                    ValidationRule {
                        rule_type: ValidationType::Compilation,
                        description: "确保代码编译通过".to_string(),
                        command: Some("cargo build".to_string()),
                        expected_result: Some("编译成功".to_string()),
                    }
                ],
                estimated_duration: Some(30),
            },
        ]
    }

    /// 生成简单的操作
    fn generate_simple_actions(&self, description: &str) -> Vec<PlanAction> {
        vec![
            PlanAction::CreateBranch {
                name: crate::commands::branch::generate_smart_branch_name(description),
                from_branch: None,
            },
            PlanAction::CreateFile {
                path: "src/new_feature.rs".to_string(),
                content: format!(
                    "// TODO: 实现 {}\n\npub fn main() {{\n    println!(\"Hello, {}!\");\n}}",
                    description, description
                ),
                template: None,
            },
            PlanAction::UpdateChangelog {
                entry: format!("添加功能: {}", description),
                version: None,
            },
        ]
    }

    /// 创建简单分析
    fn create_simple_analysis(&self, description: &str) -> PlanAnalysis {
        PlanAnalysis {
            related_files: vec![],
            code_understanding: vec![], // 简单模式不进行代码理解
            architecture_notes: format!("简单模式分析: {}", description),
            dependency_graph: DependencyGraph {
                nodes: vec![],
                edges: vec![],
            }, // 简单模式不分析依赖
        }
    }

    /// 创建简单技术方案
    fn create_simple_solution(&self, description: &str) -> TechnicalSolution {
        TechnicalSolution {
            approach: format!("简单实现方案: {}", description),
            architecture_pattern: None,
            design_principles: vec!["保持简单".to_string()],
            implementation_steps: vec![
                ImplementationStep {
                    step_number: 1,
                    title: "创建基础文件".to_string(),
                    description: "创建必要的源文件".to_string(),
                    code_snippets: vec![],
                    files_to_modify: vec!["src/new_feature.rs".to_string()],
                }
            ],
            alternatives_considered: vec![],
            risks_and_mitigations: vec![],
        }
    }

    /// 创建简单影响评估
    fn create_simple_impact(&self) -> ImpactAssessment {
        ImpactAssessment {
            affected_components: vec![],
            breaking_changes: vec![],
            testing_requirements: vec![],
            performance_impact: PerformanceImpact {
                expected_change: PerformanceChange::Neutral,
                metrics_affected: vec![],
                benchmarking_plan: None,
            },
            security_considerations: vec![],
        }
    }

    /// 创建简单元数据
    fn create_simple_metadata(&self) -> PlanMetadata {
        PlanMetadata {
            technical_approach: "简单模板生成".to_string(),
            architecture_notes: "基础实现，未进行深度分析".to_string(),
            dependencies: vec![],
            estimated_complexity: ComplexityLevel::Low,
            related_files: vec![],
        }
    }

    /// 创建简单项目上下文
    async fn create_simple_context(&self) -> Result<ProjectContext> {
        Ok(ProjectContext {
            language: "Rust".to_string(), // 假设是 Rust 项目
            framework: None,
            structure: ProjectStructure {
                directories: vec!["src".to_string()],
                patterns: vec![], // 简单模式不分析模式
                entry_points: vec![], // 简单模式不分析入口点
            },
            key_files: vec![], // 简单模式不分析关键文件
            architecture_notes: "简单模式，未进行架构分析".to_string(),
        })
    }
}
