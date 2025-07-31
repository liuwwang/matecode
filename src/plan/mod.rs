use crate::config;
use crate::llm::parse_prompt_template;
use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};

use chrono::{DateTime, Utc};
use uuid::Uuid;

pub mod analyzer;
pub mod executor;
pub mod generator;
pub mod storage;

/// 计划状态
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PlanStatus {
    Draft,
    InProgress,
    Completed,
    Cancelled,
}

/// 计划操作类型 - 重新设计为更强大的操作类型
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "data")]
pub enum PlanAction {
    // Git 操作
    CreateBranch { name: String, from_branch: Option<String> },
    SwitchBranch { name: String },

    // 文件操作
    CreateFile { path: String, content: String, template: Option<String> },
    ModifyFile { path: String, changes: Vec<FileChange>, backup: bool },
    AppendToFile { path: String, content: String, position: AppendPosition },
    CreateDirectory { path: String, recursive: bool },

    // 代码操作
    GenerateCode {
        target_file: String,
        function_name: String,
        implementation: String,
        tests: Option<String>,
        documentation: Option<String>
    },
    RefactorCode {
        file_path: String,
        old_pattern: String,
        new_pattern: String,
        scope: RefactorScope
    },

    // 依赖管理
    AddDependency { name: String, version: Option<String>, dev: bool },
    UpdateDependency { name: String, version: String },

    // 文档操作
    UpdateChangelog { entry: String, version: Option<String> },
    GenerateDocumentation { target: DocumentationTarget, content: String },

    // 执行命令
    RunCommand { command: String, description: String, working_dir: Option<String> },
    RunTests { test_pattern: Option<String>, coverage: bool },

    // 验证操作
    ValidateCode { file_path: String, rules: Vec<String> },
    CheckDependencies,
}

/// 文件修改操作 - 增强版
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileChange {
    pub line_number: Option<usize>,
    pub change_type: ChangeType,
    pub content: String,
    pub context: Option<String>, // 上下文信息，帮助定位
    pub reason: Option<String>,  // 修改原因
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ChangeType {
    Insert,
    Replace,
    Delete,
    Append,
    InsertBefore,
    InsertAfter,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AppendPosition {
    End,
    BeforeLastLine,
    AfterImports,
    BeforeFunction(String),
    AfterFunction(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RefactorScope {
    Function(String),
    Class(String),
    Module,
    Global,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum DocumentationTarget {
    README,
    API,
    UserGuide,
    DeveloperGuide,
    Changelog,
}

/// 项目上下文信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectContext {
    pub language: String,
    pub framework: Option<String>,
    pub structure: ProjectStructure,
    pub key_files: Vec<FileContext>,
    pub architecture_notes: String,
}

/// 项目结构信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectStructure {
    pub directories: Vec<String>,
    pub patterns: Vec<String>,
    pub entry_points: Vec<String>,
}

/// 文件上下文信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileContext {
    pub path: String,
    pub summary: String,
    pub key_functions: Vec<String>,
    pub dependencies: Vec<String>,
}

/// 完整的开发计划 - 增强版
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Plan {
    pub id: String,
    pub title: String,
    pub description: String,
    pub branch_name: String,
    pub status: PlanStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // 核心内容
    pub phases: Vec<PlanPhase>,  // 分阶段执行
    pub actions: Vec<PlanAction>,
    pub affected_files: Vec<String>,

    // 分析结果
    pub analysis: PlanAnalysis,
    pub technical_solution: TechnicalSolution,
    pub impact_assessment: ImpactAssessment,

    // 元数据
    pub metadata: PlanMetadata,
    pub project_context: ProjectContext,

    // 执行相关
    pub execution_config: ExecutionConfig,
    pub user_preferences: UserPreferences,
}

/// 计划阶段 - 支持分阶段执行
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlanPhase {
    pub id: String,
    pub name: String,
    pub description: String,
    pub actions: Vec<usize>, // 引用 actions 的索引
    pub dependencies: Vec<String>, // 依赖的其他阶段
    pub validation_rules: Vec<ValidationRule>,
    pub estimated_duration: Option<u32>, // 预估时间（分钟）
}

/// 计划分析结果
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlanAnalysis {
    pub related_files: Vec<RelatedFile>,
    pub code_understanding: Vec<CodeUnderstanding>,
    pub dependency_graph: DependencyGraph,
    pub architecture_notes: String,
}

/// 相关文件信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RelatedFile {
    pub path: String,
    pub file_type: FileType,
    pub relevance_score: f32,
    pub summary: String,
    pub key_functions: Vec<String>,
    pub dependencies: Vec<String>,
}

/// 代码理解报告
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodeUnderstanding {
    pub file_path: String,
    pub summary: String,
    pub key_concepts: Vec<String>,
    pub patterns_identified: Vec<String>,
    pub suggestions: Vec<String>,
}

/// 依赖关系图
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DependencyGraph {
    pub nodes: Vec<DependencyNode>,
    pub edges: Vec<DependencyEdge>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DependencyNode {
    pub id: String,
    pub name: String,
    pub node_type: NodeType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DependencyEdge {
    pub from: String,
    pub to: String,
    pub relationship: RelationshipType,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum FileType {
    Source,
    Test,
    Config,
    Documentation,
    Build,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum NodeType {
    File,
    Function,
    Class,
    Module,
    Package,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RelationshipType {
    Imports,
    Calls,
    Inherits,
    Implements,
    Uses,
}

/// 技术解决方案
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TechnicalSolution {
    pub approach: String,
    pub architecture_pattern: Option<String>,
    pub design_principles: Vec<String>,
    pub implementation_steps: Vec<ImplementationStep>,
    pub alternatives_considered: Vec<Alternative>,
    pub risks_and_mitigations: Vec<RiskMitigation>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImplementationStep {
    pub step_number: usize,
    pub title: String,
    pub description: String,
    pub code_snippets: Vec<CodeSnippet>,
    pub files_to_modify: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodeSnippet {
    pub language: String,
    pub code: String,
    pub explanation: String,
    pub file_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Alternative {
    pub name: String,
    pub description: String,
    pub pros: Vec<String>,
    pub cons: Vec<String>,
    pub why_not_chosen: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RiskMitigation {
    pub risk: String,
    pub probability: RiskLevel,
    pub impact: RiskLevel,
    pub mitigation: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// 影响评估
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImpactAssessment {
    pub affected_components: Vec<ComponentImpact>,
    pub breaking_changes: Vec<BreakingChange>,
    pub performance_impact: PerformanceImpact,
    pub security_considerations: Vec<String>,
    pub testing_requirements: Vec<TestingRequirement>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComponentImpact {
    pub component: String,
    pub impact_level: ImpactLevel,
    pub changes_required: Vec<String>,
    pub migration_notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BreakingChange {
    pub description: String,
    pub affected_apis: Vec<String>,
    pub migration_guide: String,
    pub deprecation_timeline: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PerformanceImpact {
    pub expected_change: PerformanceChange,
    pub metrics_affected: Vec<String>,
    pub benchmarking_plan: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TestingRequirement {
    pub test_type: TestType,
    pub description: String,
    pub priority: Priority,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ImpactLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PerformanceChange {
    Improvement,
    Degradation,
    Neutral,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TestType {
    Unit,
    Integration,
    EndToEnd,
    Performance,
    Security,
    Compatibility,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

/// 需求分析结果 - AI 对用户需求的深度理解
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RequirementAnalysis {
    pub intent: UserIntent,
    pub scope: RequirementScope,
    pub approach: String,
    pub architecture_notes: String,
    pub dependencies: Vec<String>,
    pub complexity: ComplexityLevel,
    pub key_components: Vec<ComponentRequirement>,
    pub constraints: Vec<String>,
}

/// 用户意图分析
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserIntent {
    pub primary_goal: String,
    pub secondary_goals: Vec<String>,
    pub user_type: UserType,
    pub urgency: UrgencyLevel,
}

/// 需求范围
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RequirementScope {
    pub feature_type: FeatureType,
    pub affected_layers: Vec<ArchitectureLayer>,
    pub integration_points: Vec<String>,
    pub external_dependencies: Vec<String>,
}

/// 组件需求
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComponentRequirement {
    pub name: String,
    pub purpose: String,
    pub interfaces: Vec<String>,
    pub dependencies: Vec<String>,
    pub estimated_effort: EffortLevel,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum UserType {
    Developer,
    EndUser,
    Administrator,
    System,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum UrgencyLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FeatureType {
    NewFeature,
    Enhancement,
    BugFix,
    Refactoring,
    Configuration,
    Documentation,
    Testing,
    Performance,
    Security,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ArchitectureLayer {
    Presentation,
    Business,
    Data,
    Infrastructure,
    Configuration,
    Testing,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum EffortLevel {
    Trivial,    // < 1 hour
    Small,      // 1-4 hours
    Medium,     // 4-16 hours
    Large,      // 1-3 days
    ExtraLarge, // > 3 days
}

/// 执行配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExecutionConfig {
    pub auto_confirm: bool,
    pub backup_files: bool,
    pub dry_run: bool,
    pub parallel_execution: bool,
    pub max_retries: u32,
    pub timeout_seconds: u32,
    pub rollback_on_failure: bool,
}

/// 用户偏好
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserPreferences {
    pub preferred_editor: Option<String>,
    pub code_style: CodeStyle,
    pub notification_level: NotificationLevel,
    pub auto_format: bool,
    pub generate_tests: bool,
    pub update_documentation: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodeStyle {
    pub indentation: IndentationType,
    pub line_length: u32,
    pub naming_convention: NamingConvention,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum IndentationType {
    Spaces(u8),
    Tabs,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum NamingConvention {
    CamelCase,
    SnakeCase,
    KebabCase,
    PascalCase,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum NotificationLevel {
    Silent,
    Errors,
    Warnings,
    Info,
    Verbose,
}

/// 验证规则
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidationRule {
    pub rule_type: ValidationType,
    pub description: String,
    pub command: Option<String>,
    pub expected_result: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ValidationType {
    Compilation,
    Tests,
    Linting,
    TypeCheck,
    Security,
    Performance,
    Custom(String),
}

/// 计划元数据
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlanMetadata {
    pub technical_approach: String,
    pub architecture_notes: String,
    pub dependencies: Vec<String>,
    pub estimated_complexity: ComplexityLevel,
    pub related_files: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ComplexityLevel {
    Low,
    Medium,
    High,
    VeryHigh,
}

/// 计划生成器
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
    pub async fn generate_plan(&self, description: &str) -> Result<Plan> {
        self.generate_plan_with_context_management(description, false).await
    }

    /// 生成计划，支持上下文压缩
    pub async fn generate_plan_with_context_management(&self, description: &str, use_compressed: bool) -> Result<Plan> {
        // 根据是否需要压缩来选择不同的上下文获取方式
        let (project_context, related_files) = if use_compressed {
            // 使用压缩的上下文
            let project_context = self.project_analyzer.get_compressed_context(5).await?;
            let related_files = self.project_analyzer.get_essential_related_files(description, 3).await?;
            (project_context, related_files)
        } else {
            // 使用完整的上下文
            let project_context = self.project_analyzer.analyze_codebase().await?;
            let related_files = self.project_analyzer.find_related_files(description).await?;
            (project_context, related_files)
        };

        // 生成完整计划（包括分支名、技术方案、操作列表）
        let plan = self.generate_comprehensive_plan(description, &project_context, &related_files).await?;

        Ok(plan)
    }

    async fn generate_comprehensive_plan(
        &self,
        description: &str,
        project_context: &ProjectContext,
        related_files: &[FileContext],
    ) -> Result<Plan> {
        // 获取 LLM 客户端
        let llm_client = config::get_llm_client().await?;

        // 构建包含所有上下文的 prompt
        let template = config::get_prompt_template("plan").await?;
        let (system_prompt, user_prompt) = parse_prompt_template(&template)?;

        let user_prompt = self.build_plan_prompt(
            &user_prompt,
            description,
            project_context,
            related_files,
        );

        println!("🤖 正在调用 AI 生成详细计划...");
        println!("📡 发送请求到 LLM API (这可能需要几秒钟)...");

        let response = llm_client.as_client().call(&system_prompt, &user_prompt).await?;

        println!("✅ 收到 AI 响应，正在解析计划结构...");

        // 解析 LLM 响应为结构化的 Plan
        self.parse_plan_response(&response, description).await
    }
    
    fn build_plan_prompt(
        &self,
        template: &str,
        description: &str,
        project_context: &ProjectContext,
        related_files: &[FileContext],
    ) -> String {
        let related_files_summary = related_files
            .iter()
            .map(|f| format!("- {}: {}", f.path, f.summary))
            .collect::<Vec<_>>()
            .join("\n");
            
        template
            .replace("{description}", description)
            .replace("{project_language}", &project_context.language)
            .replace("{project_framework}", &project_context.framework.as_deref().unwrap_or("未知"))
            .replace("{project_structure}", &serde_json::to_string_pretty(&project_context.structure).unwrap_or_default())
            .replace("{related_files}", &related_files_summary)
            .replace("{architecture_notes}", &project_context.architecture_notes)
    }
    
    async fn parse_plan_response(&self, response: &str, description: &str) -> Result<Plan> {
        // 尝试从响应中提取 XML
        let xml_str = self.extract_xml_from_response(response)?;

        // 解析 XML 为临时结构
        let plan_response = self.parse_xml_plan(&xml_str)?;

        // 转换为完整的 Plan 结构
        let plan = self.convert_response_to_plan(plan_response, description).await?;

        Ok(plan)
    }

    /// 从 LLM 响应中提取 XML 内容
    fn extract_xml_from_response(&self, response: &str) -> Result<String> {
        // 查找 XML 代码块
        if let Some(start) = response.find("```xml") {
            let start = start + 6; // "```xml".len()
            if let Some(end) = response[start..].find("```") {
                return Ok(response[start..start + end].trim().to_string());
            }
        }

        // 如果没有找到代码块，尝试查找 <plan> 标签
        if let Some(start) = response.find("<plan>") {
            if let Some(end) = response.find("</plan>") {
                let end = end + 7; // "</plan>".len()
                return Ok(response[start..end].to_string());
            }
        }

        Err(anyhow!("无法从响应中提取有效的 XML 内容"))
    }

    /// 解析 XML 计划内容
    fn parse_xml_plan(&self, xml_str: &str) -> Result<PlanResponse> {
        // 清理和修复 XML 内容
        let cleaned_xml = self.clean_xml_content(xml_str);

        // 尝试手动解析 XML（更容错）
        self.parse_xml_manually(&cleaned_xml)
    }

    /// 清理 XML 内容，处理常见的格式问题
    fn clean_xml_content(&self, xml_str: &str) -> String {
        let mut cleaned = xml_str.to_string();

        // 修复常见的 XML 格式问题
        // 1. 确保 plan 标签存在
        if !cleaned.contains("<plan>") && !cleaned.contains("</plan>") {
            // 如果没有 plan 标签，尝试添加
            if !cleaned.starts_with("<plan>") {
                cleaned = format!("<plan>\n{}\n</plan>", cleaned);
            }
        }

        // 2. 修复未闭合的标签（简单情况）
        let tags_to_check = vec![
            "branch_name", "technical_approach", "complexity",
            "actions", "affected_files", "dependencies", "implementation_notes"
        ];

        for tag in tags_to_check {
            let open_tag = format!("<{}>", tag);
            let close_tag = format!("</{}>", tag);

            // 如果有开始标签但没有结束标签，尝试添加
            if cleaned.contains(&open_tag) && !cleaned.contains(&close_tag) {
                // 简单的修复：在最后添加结束标签
                if let Some(pos) = cleaned.rfind(&open_tag) {
                    if let Some(next_tag_pos) = cleaned[pos..].find('<') {
                        if next_tag_pos > open_tag.len() {
                            // 在下一个标签前插入结束标签
                            let insert_pos = pos + next_tag_pos;
                            cleaned.insert_str(insert_pos, &close_tag);
                        }
                    }
                }
            }
        }

        cleaned
    }

    /// 手动解析 XML（更容错的方式）
    fn parse_xml_manually(&self, xml_str: &str) -> Result<PlanResponse> {
        let branch_name = self.extract_xml_tag_content(xml_str, "branch_name")?;
        let technical_approach = self.extract_xml_tag_content(xml_str, "technical_approach")?;
        let complexity = self.extract_xml_tag_content(xml_str, "complexity").unwrap_or_else(|_| "Medium".to_string());
        let implementation_notes = self.extract_xml_tag_content(xml_str, "implementation_notes").ok();

        // 解析 actions
        let actions = self.parse_actions_manually(xml_str)?;

        // 解析 affected_files
        let affected_files = self.parse_file_list_manually(xml_str, "affected_files")?;

        // 解析 dependencies
        let dependencies = self.parse_file_list_manually(xml_str, "dependencies").unwrap_or_else(|_| FileList { files: vec![] });

        Ok(PlanResponse {
            branch_name,
            technical_approach,
            complexity,
            actions,
            affected_files,
            dependencies: DependencyList { dependencies: dependencies.files },
            implementation_notes,
        })
    }

    /// 提取 XML 标签内容
    fn extract_xml_tag_content(&self, xml_str: &str, tag: &str) -> Result<String> {
        let start_tag = format!("<{}>", tag);
        let end_tag = format!("</{}>", tag);

        if let Some(start) = xml_str.find(&start_tag) {
            let start = start + start_tag.len();
            if let Some(end) = xml_str[start..].find(&end_tag) {
                return Ok(xml_str[start..start + end].trim().to_string());
            }
        }

        Err(anyhow!("无法找到标签 {} 的内容", tag))
    }

    /// 手动解析 actions
    fn parse_actions_manually(&self, xml_str: &str) -> Result<ActionList> {
        let mut actions = Vec::new();

        // 查找所有 <action> 标签
        let mut search_start = 0;
        while let Some(action_start) = xml_str[search_start..].find("<action") {
            let action_start = search_start + action_start;
            if let Some(action_end) = xml_str[action_start..].find("</action>") {
                let action_end = action_start + action_end + 9; // "</action>".len()
                let action_xml = &xml_str[action_start..action_end];

                if let Ok(action) = self.parse_single_action(action_xml) {
                    actions.push(action);
                }

                search_start = action_end;
            } else {
                break;
            }
        }

        Ok(ActionList { actions })
    }

    /// 解析单个 action
    fn parse_single_action(&self, action_xml: &str) -> Result<XmlAction> {
        // 提取 type 属性
        let action_type = if let Some(type_start) = action_xml.find("type=\"") {
            let type_start = type_start + 6; // "type=\"".len()
            if let Some(type_end) = action_xml[type_start..].find("\"") {
                action_xml[type_start..type_start + type_end].to_string()
            } else {
                return Err(anyhow!("无法解析 action type"));
            }
        } else {
            return Err(anyhow!("action 缺少 type 属性"));
        };

        // 提取各种可能的子标签内容
        let name = self.extract_xml_tag_content(action_xml, "name").ok();
        let path = self.extract_xml_tag_content(action_xml, "path").ok();
        let content = self.extract_xml_tag_content(action_xml, "content").ok();
        let command = self.extract_xml_tag_content(action_xml, "command").ok();
        let description = self.extract_xml_tag_content(action_xml, "description").ok();
        let entry = self.extract_xml_tag_content(action_xml, "entry").ok();

        Ok(XmlAction {
            action_type,
            name,
            path,
            content,
            command,
            description,
            entry,
            changes: None, // 暂时简化，不解析 changes
        })
    }

    /// 解析文件列表
    fn parse_file_list_manually(&self, xml_str: &str, list_tag: &str) -> Result<FileList> {
        let start_tag = format!("<{}>", list_tag);
        let end_tag = format!("</{}>", list_tag);

        if let Some(start) = xml_str.find(&start_tag) {
            let start = start + start_tag.len();
            if let Some(end) = xml_str[start..].find(&end_tag) {
                let list_content = &xml_str[start..start + end];

                let mut files = Vec::new();
                let mut search_start = 0;

                while let Some(file_start) = list_content[search_start..].find("<file>") {
                    let file_start = search_start + file_start + 6; // "<file>".len()
                    if let Some(file_end) = list_content[file_start..].find("</file>") {
                        let file_content = list_content[file_start..file_start + file_end].trim().to_string();
                        files.push(file_content);
                        search_start = file_start + file_end + 7; // "</file>".len()
                    } else {
                        break;
                    }
                }

                return Ok(FileList { files });
            }
        }

        Ok(FileList { files: vec![] })
    }

    /// 将响应转换为完整的 Plan 结构
    async fn convert_response_to_plan(&self, response: PlanResponse, description: &str) -> Result<Plan> {
        let project_context = self.project_analyzer.analyze_codebase().await?;

        // 转换 actions
        let actions = self.convert_xml_actions(response.actions)?;

        // 转换复杂度
        let complexity = self.parse_complexity(&response.complexity)?;

        Ok(Plan {
            id: Uuid::new_v4().to_string(),
            title: description.to_string(),
            description: description.to_string(),
            branch_name: response.branch_name,
            status: PlanStatus::Draft,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            phases: vec![], // 暂时为空，稍后实现
            actions,
            affected_files: response.affected_files.files,
            analysis: PlanAnalysis::default(),
            technical_solution: TechnicalSolution::default(),
            impact_assessment: ImpactAssessment::default(),
            metadata: PlanMetadata {
                technical_approach: response.technical_approach,
                architecture_notes: response.implementation_notes.unwrap_or_default(),
                dependencies: response.dependencies.dependencies,
                estimated_complexity: complexity,
                related_files: vec![], // 可以从 project_context 中获取
            },
            project_context,
            execution_config: ExecutionConfig::default(),
            user_preferences: UserPreferences::default(),
        })
    }

    /// 转换 XML actions 为 PlanAction
    fn convert_xml_actions(&self, action_list: ActionList) -> Result<Vec<PlanAction>> {
        let mut actions = Vec::new();

        for xml_action in action_list.actions {
            let action = match xml_action.action_type.as_str() {
                "CreateBranch" => {
                    let name = xml_action.name.ok_or_else(|| anyhow!("CreateBranch action missing name"))?;
                    PlanAction::CreateBranch { name, from_branch: None }
                }
                "CreateFile" => {
                    let path = xml_action.path.ok_or_else(|| anyhow!("CreateFile action missing path"))?;
                    let content = xml_action.content.unwrap_or_default();
                    PlanAction::CreateFile { path, content, template: None }
                }
                "CreateDirectory" => {
                    let path = xml_action.path.ok_or_else(|| anyhow!("CreateDirectory action missing path"))?;
                    PlanAction::CreateDirectory { path, recursive: true }
                }
                "RunCommand" => {
                    let command = xml_action.command.ok_or_else(|| anyhow!("RunCommand action missing command"))?;
                    let description = xml_action.description.unwrap_or_default();
                    PlanAction::RunCommand { command, description, working_dir: None }
                }
                "AddToChangelog" => {
                    let entry = xml_action.entry.ok_or_else(|| anyhow!("AddToChangelog action missing entry"))?;
                    PlanAction::UpdateChangelog { entry, version: None }
                }
                "ModifyFile" => {
                    let path = xml_action.path.ok_or_else(|| anyhow!("ModifyFile action missing path"))?;
                    let changes = if let Some(change_list) = xml_action.changes {
                        self.convert_xml_changes(change_list)?
                    } else {
                        vec![]
                    };
                    PlanAction::ModifyFile { path, changes, backup: true }
                }
                _ => {
                    return Err(anyhow!("未知的 action 类型: {}", xml_action.action_type));
                }
            };
            actions.push(action);
        }

        Ok(actions)
    }

    /// 转换 XML changes 为 FileChange
    fn convert_xml_changes(&self, change_list: ChangeList) -> Result<Vec<FileChange>> {
        let mut changes = Vec::new();

        for xml_change in change_list.changes {
            let line_number = xml_change.line.and_then(|s| s.parse().ok());
            let change_type = match xml_change.change_type.as_str() {
                "Insert" => ChangeType::Insert,
                "Replace" => ChangeType::Replace,
                "Delete" => ChangeType::Delete,
                "Append" => ChangeType::Append,
                _ => ChangeType::Insert, // 默认为插入
            };

            changes.push(FileChange {
                line_number,
                change_type,
                content: xml_change.content,
                context: None,
                reason: None,
            });
        }

        Ok(changes)
    }

    /// 解析复杂度字符串
    fn parse_complexity(&self, complexity_str: &str) -> Result<ComplexityLevel> {
        match complexity_str.to_lowercase().as_str() {
            "low" => Ok(ComplexityLevel::Low),
            "medium" => Ok(ComplexityLevel::Medium),
            "high" => Ok(ComplexityLevel::High),
            "veryhigh" => Ok(ComplexityLevel::VeryHigh),
            _ => Ok(ComplexityLevel::Medium), // 默认为中等
        }
    }
}

/// LLM 响应的临时结构
#[derive(Debug, Deserialize)]
#[serde(rename = "plan")]
struct PlanResponse {
    branch_name: String,
    technical_approach: String,
    complexity: String, // 改为 String，稍后转换
    actions: ActionList,
    affected_files: FileList,
    dependencies: DependencyList,
    implementation_notes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ActionList {
    #[serde(rename = "action")]
    actions: Vec<XmlAction>,
}

#[derive(Debug, Deserialize)]
struct XmlAction {
    #[serde(rename = "@type")]
    action_type: String,
    name: Option<String>,
    path: Option<String>,
    content: Option<String>,
    command: Option<String>,
    description: Option<String>,
    entry: Option<String>,
    changes: Option<ChangeList>,
}

#[derive(Debug, Deserialize)]
struct ChangeList {
    #[serde(rename = "change")]
    changes: Vec<XmlChange>,
}

#[derive(Debug, Deserialize)]
struct XmlChange {
    #[serde(rename = "@line")]
    line: Option<String>,
    #[serde(rename = "@type")]
    change_type: String,
    #[serde(rename = "$text")]
    content: String,
}

#[derive(Debug, Deserialize)]
struct FileList {
    #[serde(rename = "file")]
    files: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct DependencyList {
    #[serde(rename = "dependency")]
    dependencies: Vec<String>,
}

// 导出存储相关类型
pub use storage::{PlanStorage, StoredPlan};

// 默认实现，用于向后兼容
impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            auto_confirm: false,
            backup_files: true,
            dry_run: false,
            parallel_execution: false,
            max_retries: 3,
            timeout_seconds: 300,
            rollback_on_failure: true,
        }
    }
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            preferred_editor: None,
            code_style: CodeStyle::default(),
            notification_level: NotificationLevel::Info,
            auto_format: true,
            generate_tests: false,
            update_documentation: true,
        }
    }
}

impl Default for CodeStyle {
    fn default() -> Self {
        Self {
            indentation: IndentationType::Spaces(4),
            line_length: 100,
            naming_convention: NamingConvention::SnakeCase,
        }
    }
}

impl Default for PlanAnalysis {
    fn default() -> Self {
        Self {
            related_files: vec![],
            code_understanding: vec![],
            dependency_graph: DependencyGraph {
                nodes: vec![],
                edges: vec![],
            },
            architecture_notes: String::new(),
        }
    }
}

impl Default for TechnicalSolution {
    fn default() -> Self {
        Self {
            approach: String::new(),
            architecture_pattern: None,
            design_principles: vec![],
            implementation_steps: vec![],
            alternatives_considered: vec![],
            risks_and_mitigations: vec![],
        }
    }
}

impl Default for ImpactAssessment {
    fn default() -> Self {
        Self {
            affected_components: vec![],
            breaking_changes: vec![],
            performance_impact: PerformanceImpact {
                expected_change: PerformanceChange::Unknown,
                metrics_affected: vec![],
                benchmarking_plan: None,
            },
            security_considerations: vec![],
            testing_requirements: vec![],
        }
    }
}
