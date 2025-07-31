use super::*;
use anyhow::{Result, anyhow};
use regex::Regex;
use std::path::Path;

/// Python 语言分析器
pub struct PythonAnalyzer {
    class_regex: Regex,
    function_regex: Regex,
    method_regex: Regex,
    import_regex: Regex,
    from_import_regex: Regex,
    variable_regex: Regex,
    decorator_regex: Regex,
}

impl PythonAnalyzer {
    pub fn new() -> Self {
        Self {
            class_regex: Regex::new(r"^(\s*)class\s+(\w+)(?:\(([^)]*)\))?:").unwrap(),
            function_regex: Regex::new(r"^(\s*)def\s+(\w+)\s*\(([^)]*)\)(?:\s*->\s*([^:]+))?:").unwrap(),
            method_regex: Regex::new(r"^(\s+)def\s+(\w+)\s*\(([^)]*)\)(?:\s*->\s*([^:]+))?:").unwrap(),
            import_regex: Regex::new(r"^import\s+(.+)").unwrap(),
            from_import_regex: Regex::new(r"^from\s+(\S+)\s+import\s+(.+)").unwrap(),
            variable_regex: Regex::new(r"^(\s*)(\w+)\s*[:=]\s*(.+)").unwrap(),
            decorator_regex: Regex::new(r"^(\s*)@(\w+)").unwrap(),
        }
    }
    
    /// 解析函数参数
    fn parse_parameters(&self, params_str: &str) -> Vec<Parameter> {
        if params_str.trim().is_empty() {
            return vec![];
        }
        
        params_str
            .split(',')
            .map(|param| {
                let param = param.trim();
                let parts: Vec<&str> = param.split(':').collect();
                let name_default: Vec<&str> = parts[0].split('=').collect();
                
                let name = name_default[0].trim().to_string();
                let default_value = if name_default.len() > 1 {
                    Some(name_default[1].trim().to_string())
                } else {
                    None
                };
                
                let param_type = if parts.len() > 1 {
                    Some(parts[1].split('=').next().unwrap().trim().to_string())
                } else {
                    None
                };
                
                Parameter {
                    name,
                    param_type,
                    is_optional: default_value.is_some(),
                    default_value,
                }
            })
            .collect()
    }
    
    /// 获取缩进级别
    fn get_indent_level(&self, line: &str) -> usize {
        line.len() - line.trim_start().len()
    }
    
    /// 检测可见性（Python 约定）
    fn detect_visibility(&self, name: &str) -> Visibility {
        if name.starts_with("__") && name.ends_with("__") {
            Visibility::Public // 魔术方法是公开的
        } else if name.starts_with("__") {
            Visibility::Private // 双下划线开头是私有的
        } else if name.starts_with('_') {
            Visibility::Protected // 单下划线开头是受保护的
        } else {
            Visibility::Public
        }
    }
    
    /// 提取文档字符串
    fn extract_docstring(&self, lines: &[&str], start_line: usize) -> Option<String> {
        if start_line + 1 >= lines.len() {
            return None;
        }
        
        let next_line = lines[start_line + 1].trim();
        if next_line.starts_with("\"\"\"") || next_line.starts_with("'''") {
            let quote = if next_line.starts_with("\"\"\"") { "\"\"\"" } else { "'''" };
            
            // 单行文档字符串
            if next_line.ends_with(quote) && next_line.len() > 6 {
                return Some(next_line[3..next_line.len()-3].to_string());
            }
            
            // 多行文档字符串
            let mut docstring = String::new();
            for i in (start_line + 1)..lines.len() {
                let line = lines[i].trim();
                if line.ends_with(quote) {
                    docstring.push_str(&line[..line.len()-3]);
                    break;
                }
                if i == start_line + 1 {
                    docstring.push_str(&line[3..]);
                } else {
                    docstring.push_str(line);
                }
                docstring.push('\n');
            }
            
            if !docstring.is_empty() {
                return Some(docstring.trim().to_string());
            }
        }
        
        None
    }
}

impl LanguageAnalyzer for PythonAnalyzer {
    fn analyze_file(&self, file_path: &Path, content: &str) -> Result<CodeStructure> {
        let symbols = self.extract_symbols(content)?;
        let dependencies = self.extract_dependencies(content, file_path)?;
        let imports = self.extract_imports(content)?;
        let exports = self.extract_exports(content)?;
        let complexity_score = self.calculate_complexity(content)?;
        
        Ok(CodeStructure {
            language: Language::Python,
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
        let mut current_class: Option<String> = None;
        let mut class_indent = 0;
        
        for (line_num, line) in lines.iter().enumerate() {
            let line_number = line_num + 1;
            
            // 检测类定义
            if let Some(caps) = self.class_regex.captures(line) {
                let indent = caps.get(1).unwrap().as_str().len();
                let class_name = caps.get(2).unwrap().as_str().to_string();
                let parent_classes = caps.get(3).map(|m| m.as_str().to_string());
                
                current_class = Some(class_name.clone());
                class_indent = indent;
                
                let mut attributes = HashMap::new();
                if let Some(parents) = parent_classes {
                    attributes.insert("inheritance".to_string(), parents);
                }
                
                symbols.push(Symbol {
                    name: class_name,
                    symbol_type: SymbolType::Class,
                    line_number,
                    column: indent,
                    visibility: self.detect_visibility(&caps.get(2).unwrap().as_str()),
                    documentation: self.extract_docstring(&lines, line_num),
                    parameters: vec![],
                    return_type: None,
                    parent: None,
                    attributes,
                });
            }
            // 检测函数/方法定义
            else if let Some(caps) = self.function_regex.captures(line) {
                let indent = caps.get(1).unwrap().as_str().len();
                let func_name = caps.get(2).unwrap().as_str().to_string();
                let params_str = caps.get(3).unwrap().as_str();
                let return_type = caps.get(4).map(|m| m.as_str().trim().to_string());
                
                // 判断是否在类内部（方法）
                let (symbol_type, parent) = if let Some(ref class_name) = current_class {
                    if indent > class_indent {
                        (SymbolType::Method, Some(class_name.clone()))
                    } else {
                        current_class = None;
                        (SymbolType::Function, None)
                    }
                } else {
                    (SymbolType::Function, None)
                };
                
                symbols.push(Symbol {
                    name: func_name.clone(),
                    symbol_type,
                    line_number,
                    column: indent,
                    visibility: self.detect_visibility(&func_name),
                    documentation: self.extract_docstring(&lines, line_num),
                    parameters: self.parse_parameters(params_str),
                    return_type,
                    parent,
                    attributes: HashMap::new(),
                });
            }
            // 检测变量定义
            else if let Some(caps) = self.variable_regex.captures(line) {
                let indent = caps.get(1).unwrap().as_str().len();
                let var_name = caps.get(2).unwrap().as_str().to_string();
                let value = caps.get(3).unwrap().as_str().to_string();
                
                // 跳过函数内的局部变量（简单启发式）
                if indent == 0 || (current_class.is_some() && indent <= class_indent + 4) {
                    let symbol_type = if var_name.chars().all(|c| c.is_uppercase() || c == '_') {
                        SymbolType::Constant
                    } else {
                        SymbolType::Variable
                    };
                    
                    let mut attributes = HashMap::new();
                    attributes.insert("value".to_string(), value);
                    
                    symbols.push(Symbol {
                        name: var_name.clone(),
                        symbol_type,
                        line_number,
                        column: indent,
                        visibility: self.detect_visibility(&var_name),
                        documentation: None,
                        parameters: vec![],
                        return_type: None,
                        parent: current_class.clone(),
                        attributes,
                    });
                }
            }
        }
        
        Ok(symbols)
    }
    
    fn extract_dependencies(&self, content: &str, _file_path: &Path) -> Result<Vec<Dependency>> {
        let mut dependencies = Vec::new();
        
        for (line_num, line) in content.lines().enumerate() {
            let line_number = line_num + 1;
            
            // import module
            if let Some(caps) = self.import_regex.captures(line) {
                let modules = caps.get(1).unwrap().as_str();
                for module in modules.split(',') {
                    let module = module.trim();
                    dependencies.push(Dependency {
                        name: module.to_string(),
                        dependency_type: DependencyType::Import,
                        source: "current_file".to_string(),
                        target: module.to_string(),
                        line_number,
                    });
                }
            }
            // from module import items
            else if let Some(caps) = self.from_import_regex.captures(line) {
                let module = caps.get(1).unwrap().as_str();
                let items = caps.get(2).unwrap().as_str();
                
                dependencies.push(Dependency {
                    name: format!("{}.{}", module, items),
                    dependency_type: DependencyType::Import,
                    source: "current_file".to_string(),
                    target: module.to_string(),
                    line_number,
                });
            }
        }
        
        Ok(dependencies)
    }
    
    fn extract_imports(&self, content: &str) -> Result<Vec<String>> {
        let mut imports = Vec::new();
        
        for line in content.lines() {
            if let Some(caps) = self.import_regex.captures(line) {
                imports.push(caps.get(1).unwrap().as_str().to_string());
            } else if let Some(caps) = self.from_import_regex.captures(line) {
                imports.push(format!("{}.{}", caps.get(1).unwrap().as_str(), caps.get(2).unwrap().as_str()));
            }
        }
        
        Ok(imports)
    }
    
    fn extract_exports(&self, content: &str) -> Result<Vec<String>> {
        let mut exports = Vec::new();
        
        // 查找 __all__ 定义
        for line in content.lines() {
            if line.trim().starts_with("__all__") {
                // 简单解析 __all__ 列表
                if let Some(start) = line.find('[') {
                    if let Some(end) = line.find(']') {
                        let items = &line[start+1..end];
                        for item in items.split(',') {
                            let item = item.trim().trim_matches('"').trim_matches('\'');
                            if !item.is_empty() {
                                exports.push(item.to_string());
                            }
                        }
                    }
                }
            }
        }
        
        // 如果没有 __all__，导出所有公开的符号
        if exports.is_empty() {
            let symbols = self.extract_symbols(content)?;
            for symbol in symbols {
                if symbol.visibility == Visibility::Public && symbol.parent.is_none() {
                    exports.push(symbol.name);
                }
            }
        }
        
        Ok(exports)
    }
    
    fn calculate_complexity(&self, content: &str) -> Result<f32> {
        let mut complexity = 1.0; // 基础复杂度
        
        for line in content.lines() {
            let line = line.trim();
            
            // 控制流语句增加复杂度
            if line.starts_with("if ") || line.starts_with("elif ") {
                complexity += 1.0;
            } else if line.starts_with("for ") || line.starts_with("while ") {
                complexity += 1.5;
            } else if line.starts_with("try:") || line.starts_with("except ") {
                complexity += 1.0;
            } else if line.starts_with("def ") || line.starts_with("class ") {
                complexity += 0.5;
            }
        }
        
        Ok(complexity)
    }
    
    fn supported_language(&self) -> Language {
        Language::Python
    }
    
    fn generate_code(&self, symbol_type: SymbolType, name: &str, context: &CodeGenerationContext) -> Result<String> {
        match symbol_type {
            SymbolType::Function => {
                let indent = if context.style_preferences.indentation == IndentationType::Tabs {
                    "\t".to_string()
                } else if let IndentationType::Spaces(n) = context.style_preferences.indentation {
                    " ".repeat(n)
                } else {
                    "    ".to_string()
                };
                
                let mut code = format!("def {}():\n", name);
                if context.style_preferences.generate_docstrings {
                    code.push_str(&format!("{}\"\"\"{}.\"\"\"\n", indent, context.purpose));
                }
                code.push_str(&format!("{}pass\n", indent));
                
                Ok(code)
            }
            SymbolType::Class => {
                let mut code = format!("class {}:\n", name);
                if context.style_preferences.generate_docstrings {
                    code.push_str(&format!("    \"\"\"{}.\"\"\"\n", context.purpose));
                }
                code.push_str("    pass\n");
                
                Ok(code)
            }
            _ => Err(anyhow!("Unsupported symbol type for Python: {:?}", symbol_type))
        }
    }
    
    fn validate_syntax(&self, content: &str) -> Result<Vec<SyntaxError>> {
        // 简单的语法检查（实际应该使用 Python AST）
        let mut errors = Vec::new();
        
        for (line_num, line) in content.lines().enumerate() {
            // 检查缩进一致性
            if line.trim().is_empty() {
                continue;
            }
            
            let leading_spaces = line.len() - line.trim_start().len();
            if leading_spaces % 4 != 0 && !line.trim_start().starts_with('\t') {
                errors.push(SyntaxError {
                    line: line_num + 1,
                    column: 1,
                    message: "Inconsistent indentation".to_string(),
                    severity: ErrorSeverity::Warning,
                });
            }
        }
        
        Ok(errors)
    }
}

impl Default for PythonAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
