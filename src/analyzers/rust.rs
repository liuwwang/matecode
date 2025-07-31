use super::*;
use anyhow::{Result, anyhow};
use regex::Regex;
use std::path::Path;

/// Rust 语言分析器
pub struct RustAnalyzer {
    struct_regex: Regex,
    enum_regex: Regex,
    trait_regex: Regex,
    impl_regex: Regex,
    function_regex: Regex,
    use_regex: Regex,
    mod_regex: Regex,
    const_regex: Regex,
}

impl RustAnalyzer {
    pub fn new() -> Self {
        Self {
            struct_regex: Regex::new(r"^(\s*)(?:pub\s+)?struct\s+(\w+)").unwrap(),
            enum_regex: Regex::new(r"^(\s*)(?:pub\s+)?enum\s+(\w+)").unwrap(),
            trait_regex: Regex::new(r"^(\s*)(?:pub\s+)?trait\s+(\w+)").unwrap(),
            impl_regex: Regex::new(r"^(\s*)impl(?:\s*<[^>]*>)?\s+(?:(\w+)\s+for\s+)?(\w+)").unwrap(),
            function_regex: Regex::new(r"^(\s*)(?:pub\s+)?(?:async\s+)?fn\s+(\w+)\s*\(([^)]*)\)(?:\s*->\s*([^{]+))?").unwrap(),
            use_regex: Regex::new(r"^use\s+(.+);").unwrap(),
            mod_regex: Regex::new(r"^(?:pub\s+)?mod\s+(\w+)").unwrap(),
            const_regex: Regex::new(r"^(\s*)(?:pub\s+)?const\s+(\w+):\s*([^=]+)").unwrap(),
        }
    }
    
    /// 检测可见性
    fn detect_visibility(&self, line: &str) -> Visibility {
        if line.trim().starts_with("pub ") {
            Visibility::Public
        } else {
            Visibility::Private
        }
    }
    
    /// 解析函数参数
    fn parse_rust_parameters(&self, params_str: &str) -> Vec<Parameter> {
        if params_str.trim().is_empty() {
            return vec![];
        }
        
        params_str
            .split(',')
            .filter_map(|param| {
                let param = param.trim();
                if param.is_empty() {
                    return None;
                }
                
                // 处理 self 参数
                if param == "self" || param == "&self" || param == "&mut self" {
                    return Some(Parameter {
                        name: "self".to_string(),
                        param_type: Some("Self".to_string()),
                        default_value: None,
                        is_optional: false,
                    });
                }
                
                // 解析 name: type 格式
                let parts: Vec<&str> = param.split(':').collect();
                if parts.len() >= 2 {
                    let name = parts[0].trim().to_string();
                    let param_type = parts[1].trim().to_string();
                    
                    Some(Parameter {
                        name,
                        param_type: Some(param_type),
                        default_value: None,
                        is_optional: false,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// 提取文档注释
    fn extract_doc_comment(&self, lines: &[&str], line_index: usize) -> Option<String> {
        let mut doc_lines = Vec::new();
        
        // 向上查找文档注释
        for i in (0..line_index).rev() {
            let line = lines[i].trim();
            if line.starts_with("///") {
                doc_lines.insert(0, line[3..].trim().to_string());
            } else if line.starts_with("/**") && line.ends_with("*/") {
                // 单行块注释
                doc_lines.insert(0, line[3..line.len()-2].trim().to_string());
                break;
            } else if !line.is_empty() && !line.starts_with("//") && !line.starts_with("#[") {
                break;
            }
        }
        
        if doc_lines.is_empty() {
            None
        } else {
            Some(doc_lines.join(" "))
        }
    }
}

impl LanguageAnalyzer for RustAnalyzer {
    fn analyze_file(&self, file_path: &Path, content: &str) -> Result<CodeStructure> {
        let symbols = self.extract_symbols(content)?;
        let dependencies = self.extract_dependencies(content, file_path)?;
        let imports = self.extract_imports(content)?;
        let exports = self.extract_exports(content)?;
        let complexity_score = self.calculate_complexity(content)?;
        
        Ok(CodeStructure {
            language: Language::Rust,
            file_path: file_path.to_string_lossy().to_string(),
            symbols,
            dependencies,
            imports,
            exports,
            line_count: content.lines().count(),
            complexity_score,
        })
    }
    
    fn extract_symbols(&self, content: &str) -> Result<Vec<Symbol>> {
        let lines: Vec<&str> = content.lines().collect();
        let mut symbols = Vec::new();
        let mut current_impl: Option<String> = None;
        
        for (line_num, line) in lines.iter().enumerate() {
            let line_number = line_num + 1;
            
            // 检测结构体
            if let Some(caps) = self.struct_regex.captures(line) {
                let struct_name = caps.get(2).unwrap().as_str().to_string();
                
                symbols.push(Symbol {
                    name: struct_name,
                    symbol_type: SymbolType::Struct,
                    line_number,
                    column: caps.get(1).unwrap().as_str().len(),
                    visibility: self.detect_visibility(line),
                    documentation: self.extract_doc_comment(&lines, line_num),
                    parameters: vec![],
                    return_type: None,
                    parent: None,
                    attributes: HashMap::new(),
                });
            }
            // 检测枚举
            else if let Some(caps) = self.enum_regex.captures(line) {
                let enum_name = caps.get(2).unwrap().as_str().to_string();
                
                symbols.push(Symbol {
                    name: enum_name,
                    symbol_type: SymbolType::Enum,
                    line_number,
                    column: caps.get(1).unwrap().as_str().len(),
                    visibility: self.detect_visibility(line),
                    documentation: self.extract_doc_comment(&lines, line_num),
                    parameters: vec![],
                    return_type: None,
                    parent: None,
                    attributes: HashMap::new(),
                });
            }
            // 检测特质
            else if let Some(caps) = self.trait_regex.captures(line) {
                let trait_name = caps.get(2).unwrap().as_str().to_string();
                
                symbols.push(Symbol {
                    name: trait_name,
                    symbol_type: SymbolType::Trait,
                    line_number,
                    column: caps.get(1).unwrap().as_str().len(),
                    visibility: self.detect_visibility(line),
                    documentation: self.extract_doc_comment(&lines, line_num),
                    parameters: vec![],
                    return_type: None,
                    parent: None,
                    attributes: HashMap::new(),
                });
            }
            // 检测实现块
            else if let Some(caps) = self.impl_regex.captures(line) {
                let impl_target = caps.get(3).unwrap().as_str().to_string();
                current_impl = Some(impl_target);
            }
            // 检测函数
            else if let Some(caps) = self.function_regex.captures(line) {
                let func_name = caps.get(2).unwrap().as_str().to_string();
                let params_str = caps.get(3).unwrap().as_str();
                let return_type = caps.get(4).map(|m| m.as_str().trim().to_string());
                
                let symbol_type = if current_impl.is_some() {
                    SymbolType::Method
                } else {
                    SymbolType::Function
                };
                
                symbols.push(Symbol {
                    name: func_name,
                    symbol_type,
                    line_number,
                    column: caps.get(1).unwrap().as_str().len(),
                    visibility: self.detect_visibility(line),
                    documentation: self.extract_doc_comment(&lines, line_num),
                    parameters: self.parse_rust_parameters(params_str),
                    return_type,
                    parent: current_impl.clone(),
                    attributes: HashMap::new(),
                });
            }
            // 检测常量
            else if let Some(caps) = self.const_regex.captures(line) {
                let const_name = caps.get(2).unwrap().as_str().to_string();
                let const_type = caps.get(3).unwrap().as_str().trim().to_string();
                
                let mut attributes = HashMap::new();
                attributes.insert("type".to_string(), const_type);
                
                symbols.push(Symbol {
                    name: const_name,
                    symbol_type: SymbolType::Constant,
                    line_number,
                    column: caps.get(1).unwrap().as_str().len(),
                    visibility: self.detect_visibility(line),
                    documentation: self.extract_doc_comment(&lines, line_num),
                    parameters: vec![],
                    return_type: None,
                    parent: None,
                    attributes,
                });
            }
            // 检测模块
            else if let Some(caps) = self.mod_regex.captures(line) {
                let mod_name = caps.get(1).unwrap().as_str().to_string();
                
                symbols.push(Symbol {
                    name: mod_name,
                    symbol_type: SymbolType::Module,
                    line_number,
                    column: 0,
                    visibility: self.detect_visibility(line),
                    documentation: self.extract_doc_comment(&lines, line_num),
                    parameters: vec![],
                    return_type: None,
                    parent: None,
                    attributes: HashMap::new(),
                });
            }
        }
        
        Ok(symbols)
    }
    
    fn extract_dependencies(&self, content: &str, _file_path: &Path) -> Result<Vec<Dependency>> {
        let mut dependencies = Vec::new();
        
        for (line_num, line) in content.lines().enumerate() {
            if let Some(caps) = self.use_regex.captures(line) {
                let use_path = caps.get(1).unwrap().as_str();
                
                dependencies.push(Dependency {
                    name: use_path.to_string(),
                    dependency_type: DependencyType::Import,
                    source: "current_file".to_string(),
                    target: use_path.to_string(),
                    line_number: line_num + 1,
                });
            }
        }
        
        Ok(dependencies)
    }
    
    fn extract_imports(&self, content: &str) -> Result<Vec<String>> {
        let mut imports = Vec::new();
        
        for line in content.lines() {
            if let Some(caps) = self.use_regex.captures(line) {
                imports.push(caps.get(1).unwrap().as_str().to_string());
            }
        }
        
        Ok(imports)
    }
    
    fn extract_exports(&self, content: &str) -> Result<Vec<String>> {
        let mut exports = Vec::new();
        
        // 在 Rust 中，pub 项目是导出的
        let symbols = self.extract_symbols(content)?;
        for symbol in symbols {
            if symbol.visibility == Visibility::Public {
                exports.push(symbol.name);
            }
        }
        
        Ok(exports)
    }
    
    fn calculate_complexity(&self, content: &str) -> Result<f32> {
        let mut complexity = 1.0;
        
        for line in content.lines() {
            let line = line.trim();
            
            if line.starts_with("if ") || line.contains(" if ") {
                complexity += 1.0;
            } else if line.starts_with("match ") {
                complexity += 1.5;
            } else if line.starts_with("for ") || line.starts_with("while ") {
                complexity += 1.5;
            } else if line.starts_with("fn ") {
                complexity += 0.5;
            }
        }
        
        Ok(complexity)
    }
    
    fn supported_language(&self) -> Language {
        Language::Rust
    }
    
    fn generate_code(&self, symbol_type: SymbolType, name: &str, context: &CodeGenerationContext) -> Result<String> {
        match symbol_type {
            SymbolType::Function => {
                let mut code = format!("pub fn {}() {{\n", name);
                if context.style_preferences.generate_docstrings {
                    code = format!("/// {}.\npub fn {}() {{\n", context.purpose, name);
                }
                code.push_str("    todo!()\n");
                code.push_str("}\n");
                
                Ok(code)
            }
            SymbolType::Struct => {
                let mut code = if context.style_preferences.generate_docstrings {
                    format!("/// {}.\n#[derive(Debug)]\npub struct {} {{\n", context.purpose, name)
                } else {
                    format!("#[derive(Debug)]\npub struct {} {{\n", name)
                };
                code.push_str("    // TODO: Add fields\n");
                code.push_str("}\n");
                
                Ok(code)
            }
            _ => Err(anyhow!("Unsupported symbol type for Rust: {:?}", symbol_type))
        }
    }
    
    fn validate_syntax(&self, _content: &str) -> Result<Vec<SyntaxError>> {
        // Rust 语法验证应该使用 rustc 或 syn crate
        // 这里返回空列表作为占位符
        Ok(vec![])
    }
}

impl Default for RustAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
