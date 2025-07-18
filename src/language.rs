//! src/language.rs

use anyhow::Result;
use ignore::WalkBuilder;
use std::collections::HashMap;

// 定义文件扩展名到语言标识符的映射
fn get_extension_map() -> HashMap<&'static str, &'static str> {
    let mut map = HashMap::new();
    // Rust
    map.insert("rs", "rust");
    // Python
    map.insert("py", "python");
    // JavaScript & TypeScript
    map.insert("js", "javascript");
    map.insert("jsx", "javascript");
    map.insert("ts", "typescript");
    map.insert("tsx", "typescript");
    // Go
    map.insert("go", "go");
    // Java
    map.insert("java", "java");
    // C++
    map.insert("cpp", "cpp");
    map.insert("cxx", "cpp");
    map.insert("cc", "cpp");
    map.insert("hpp", "cpp");
    map.insert("hxx", "cpp");
    map.insert("hh", "cpp");
    // C
    map.insert("c", "c");
    map.insert("h", "c");
    map
}

pub fn detect_project_language() -> Result<Option<String>> {
    let mut lang_counts: HashMap<&str, usize> = HashMap::new();
    let extension_map = get_extension_map();

    let walker = WalkBuilder::new(".").build();

    for result in walker {
        let entry = match result {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        if entry.file_type().is_some_and(|ft| ft.is_file()) {
            if let Some(ext) = entry.path().extension().and_then(|s| s.to_str()) {
                if let Some(lang) = extension_map.get(ext) {
                    *lang_counts.entry(lang).or_insert(0) += 1;
                }
            }
        }
    }

    // 找到数量最多的语言
    let primary_language = lang_counts.into_iter().max_by_key(|&(_, count)| count);

    Ok(primary_language.map(|(lang, _)| lang.to_string()))
}
