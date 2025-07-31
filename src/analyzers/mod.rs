use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

pub mod rust;
pub mod python;

/// 语言类型枚举
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Rust,
    Python,
    TypeScript,
    JavaScript,
    Go,
    Java,
    CSharp,
    Unknown,
}

impl Language {
    /// 根据文件扩展名检测语言
    pub fn from_extension(extension: &str) -> Self {
        match extension.to_lowercase().as_str() {
            "rs" => Language::Rust,
            "py" | "pyw" => Language::Python,
            "ts" => Language::TypeScript,
            "js" | "jsx" => Language::JavaScript,
            "go" => Language::Go,
            "java" => Language::Java,
            "cs" => Language::CSharp,
            _ => Language::Unknown,
        }
    }
    
    /// 获取语言的主要文件扩展名
    pub fn primary_extension(&self) -> &'static str {
        match self {
            Language::Rust => "rs",
            Language::Python => "py",
            Language::TypeScript => "ts",
            Language::JavaScript => "js",
            Language::Go => "go",
            Language::Java => "java",
            Language::CSharp => "cs",
            Language::Unknown => "",
        }
    }
    
    /// 获取语言的所有支持的扩展名
    pub fn extensions(&self) -> Vec<&'static str> {
        match self {
            Language::Rust => vec!["rs"],
            Language::Python => vec!["py", "pyw"],
            Language::TypeScript => vec!["ts"],
            Language::JavaScript => vec!["js", "jsx"],
            Language::Go => vec!["go"],
            Language::Java => vec!["java"],
            Language::CSharp => vec!["cs"],
            Language::Unknown => vec![],
        }
    }
}

/// 代码符号类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SymbolType {
    Class,
    Function,
    Method,
    Variable,
    Constant,
    Interface,
    Trait,
    Struct,
    Enum,
    Module,
    Import,
    Type,
}

/// 代码符号信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub symbol_type: SymbolType,
    pub line_number: usize,
    pub column: usize,
    pub visibility: Visibility,
    pub documentation: Option<String>,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    pub parent: Option<String>, // 父级符号（如类名、模块名）
    pub attributes: HashMap<String, String>, // 语言特定的属性
}

/// 可见性
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
    Package,
}

/// 参数信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub param_type: Option<String>,
    pub default_value: Option<String>,
    pub is_optional: bool,
}

/// 依赖关系
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub dependency_type: DependencyType,
    pub source: String, // 来源文件
    pub target: String, // 目标文件或模块
    pub line_number: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DependencyType {
    Import,
    Inheritance,
    Composition,
    Usage,
    Call,
}

/// 代码结构信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeStructure {
    pub language: Language,
    pub file_path: String,
    pub symbols: Vec<Symbol>,
    pub dependencies: Vec<Dependency>,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub line_count: usize,
    pub complexity_score: f32,
}

/// 语言分析器特质
pub trait LanguageAnalyzer: Send + Sync {
    /// 分析单个文件
    fn analyze_file(&self, file_path: &Path, content: &str) -> Result<CodeStructure>;
    
    /// 提取符号
    fn extract_symbols(&self, content: &str) -> Result<Vec<Symbol>>;
    
    /// 提取依赖关系
    fn extract_dependencies(&self, content: &str, file_path: &Path) -> Result<Vec<Dependency>>;
    
    /// 提取导入语句
    fn extract_imports(&self, content: &str) -> Result<Vec<String>>;
    
    /// 提取导出语句
    fn extract_exports(&self, content: &str) -> Result<Vec<String>>;
    
    /// 计算复杂度分数
    fn calculate_complexity(&self, content: &str) -> Result<f32>;
    
    /// 获取支持的语言
    fn supported_language(&self) -> Language;
    
    /// 生成代码片段
    fn generate_code(&self, symbol_type: SymbolType, name: &str, context: &CodeGenerationContext) -> Result<String>;
    
    /// 验证代码语法
    fn validate_syntax(&self, content: &str) -> Result<Vec<SyntaxError>>;
}

/// 代码生成上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGenerationContext {
    pub target_file: String,
    pub existing_symbols: Vec<Symbol>,
    pub imports: Vec<String>,
    pub style_preferences: StylePreferences,
    pub purpose: String, // 生成代码的目的描述
}

/// 代码风格偏好
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StylePreferences {
    pub indentation: IndentationType,
    pub line_length: usize,
    pub naming_convention: NamingConvention,
    pub use_type_hints: bool,
    pub generate_docstrings: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IndentationType {
    Spaces(usize),
    Tabs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NamingConvention {
    CamelCase,
    SnakeCase,
    PascalCase,
    KebabCase,
}

/// 语法错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxError {
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub severity: ErrorSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Error,
    Warning,
    Info,
}

/// 多语言分析器管理器
pub struct LanguageAnalyzerManager {
    analyzers: HashMap<Language, Box<dyn LanguageAnalyzer>>,
}

impl LanguageAnalyzerManager {
    pub fn new() -> Self {
        let mut analyzers: HashMap<Language, Box<dyn LanguageAnalyzer>> = HashMap::new();
        
        // 注册支持的语言分析器
        analyzers.insert(Language::Rust, Box::new(rust::RustAnalyzer::new()));
        analyzers.insert(Language::Python, Box::new(python::PythonAnalyzer::new()));
        
        Self { analyzers }
    }
    
    /// 获取指定语言的分析器
    pub fn get_analyzer(&self, language: &Language) -> Option<&dyn LanguageAnalyzer> {
        self.analyzers.get(language).map(|a| a.as_ref())
    }
    
    /// 根据文件路径自动选择分析器
    pub fn get_analyzer_for_file(&self, file_path: &Path) -> Option<&dyn LanguageAnalyzer> {
        if let Some(extension) = file_path.extension().and_then(|e| e.to_str()) {
            let language = Language::from_extension(extension);
            self.get_analyzer(&language)
        } else {
            None
        }
    }
    
    /// 分析文件
    pub fn analyze_file(&self, file_path: &Path, content: &str) -> Result<CodeStructure> {
        if let Some(analyzer) = self.get_analyzer_for_file(file_path) {
            analyzer.analyze_file(file_path, content)
        } else {
            // 返回基础的结构信息
            Ok(CodeStructure {
                language: Language::Unknown,
                file_path: file_path.to_string_lossy().to_string(),
                symbols: vec![],
                dependencies: vec![],
                imports: vec![],
                exports: vec![],
                line_count: content.lines().count(),
                complexity_score: 1.0,
            })
        }
    }
    
    /// 获取所有支持的语言
    pub fn supported_languages(&self) -> Vec<Language> {
        self.analyzers.keys().cloned().collect()
    }
}

impl Default for LanguageAnalyzerManager {
    fn default() -> Self {
        Self::new()
    }
}
