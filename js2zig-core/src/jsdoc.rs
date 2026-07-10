// JSDoc 解析器（只支持 @typedef、@property、@type、@returns）
//
// 支持的 JSDoc 标签：
//   @typedef {Object} TypeName
//   @property {type} fieldName
//   @type {TypeName}
//   @returns {type}
//
// 用法：
//   let (typedefs, type_annots, return_types) = extract_all_jsdoc(js_source);
//   // typedefs:       TypeName -> TypedefDef
//   // type_annots:    var_name -> type_name  (来自 @type)
//   // return_types:   fn_name  -> type_name  (来自 @returns)

use std::collections::HashMap;

/// 一个 @typedef 定义
#[derive(Debug, Clone)]
pub struct TypedefDef {
    pub name: String,
    pub fields: Vec<TypedefField>,
}

/// @typedef 的一个 @property
#[derive(Debug, Clone)]
pub struct TypedefField {
    pub name: String,
    pub ty: String,     // JSDoc 类型字符串，如 "string"、"number"、"boolean"
    pub optional: bool, // 是否为可选属性（[name]）
}

/// 单次 JSDoc 注释的解析结果
#[derive(Debug, Default, Clone)]
pub struct ParsedJSDoc {
    /// @typedef 定义（一个注释块可以定义一个 typedef）
    pub typedef: Option<TypedefDef>,
    /// @type 标注的类型名（如 "User"）
    pub type_name: Option<String>,
    /// @returns 标注的类型名（如 "string"）
    pub return_type_name: Option<String>,
    /// @param 标注的参数类型：Vec<(参数名, 类型)>
    pub param_types: Vec<(String, String)>,
}

/// 解析单个 JSDoc 注释字符串，返回 ParsedJSDoc
pub fn parse_jsdoc(comment: &str) -> ParsedJSDoc {
    let mut result = ParsedJSDoc::default();
    let mut current_typedef: Option<TypedefDef> = None;

    for line in comment.lines() {
        let mut trimmed = line.trim();
        // 剥离单行 JSDoc 的 `/**` 前缀（如 `/** @returns {f64} */`）
        if let Some(rest) = trimmed.strip_prefix("/**") {
            trimmed = rest.trim();
        }
        // 剥离行尾的 `*/`
        if let Some(rest) = trimmed.strip_suffix("*/") {
            trimmed = rest.trim();
        }
        // 去掉开头的 * 和多余空格
        let stripped = trimmed.strip_prefix('*').unwrap_or(trimmed).trim();

        if stripped.starts_with("@typedef") {
            // 保存上一个 typedef
            if let Some(td) = current_typedef.take() {
                result.typedef = Some(td);
            }
            let rest = stripped.strip_prefix("@typedef").unwrap_or("").trim();
            let name = extract_typedef_name(rest);
            current_typedef = Some(TypedefDef {
                name,
                fields: Vec::new(),
            });
        } else if stripped.starts_with("@property") || stripped.starts_with("@prop") {
            if let Some(ref mut td) = current_typedef {
                let field = parse_property(stripped);
                td.fields.push(field);
            }
        } else if stripped.starts_with("@type") {
            let ty_name = extract_braced_type(stripped.strip_prefix("@type").unwrap_or(""));
            result.type_name = Some(ty_name);
        } else if stripped.starts_with("@returns") || stripped.starts_with("@return") {
            let prefix = if stripped.starts_with("@returns") {
                "@returns"
            } else {
                "@return"
            };
            let ty = extract_braced_type(stripped.strip_prefix(prefix).unwrap_or(""));
            result.return_type_name = Some(ty);
        } else if stripped.starts_with("@param") {
            // Parse @param {type} paramName
            let rest = stripped.strip_prefix("@param").unwrap_or("").trim();
            if rest.starts_with('{') {
                let brace_end = rest.find('}').unwrap_or(rest.len());
                let ty = rest[1..brace_end].trim().to_string();
                let after_brace = rest[brace_end + 1..].trim();
                let param_name = after_brace
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string();
                if !param_name.is_empty() {
                    result.param_types.push((param_name, ty));
                }
            }
        }
    }

    // 保存最后一个 typedef
    if let Some(td) = current_typedef.take() {
        result.typedef = Some(td);
    }

    result
}

/// 从 JS 源码中提取所有 JSDoc 注解，并关联到变量名/函数名
///
/// 返回 (typedefs, type_annotations, return_types, param_types)：
/// - typedefs:          TypeName → TypedefDef
/// - type_annotations:  var_name → type_name  （来自 @type）
/// - return_types:      fn_name  → type_name  （来自 @returns）
/// - param_types:       fn_name  → [(param_name, type)]  （来自 @param）
#[allow(clippy::type_complexity)]
pub fn extract_all_jsdoc(
    source: &str,
) -> (
    HashMap<String, TypedefDef>,
    HashMap<String, String>,
    HashMap<String, String>,
    HashMap<String, Vec<(String, String)>>,
) {
    let mut typedefs = HashMap::new();
    let mut type_annotations = HashMap::new();
    let mut return_types = HashMap::new();
    let mut param_types: HashMap<String, Vec<(String, String)>> = HashMap::new();

    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        // 识别 JSDoc 注释开始（/** ... */）
        if line.starts_with("/**") && !line.starts_with("/**/") {
            // 收集完整的 JSDoc 注释块
            let mut jsdoc_text = String::new();
            while i < lines.len() {
                jsdoc_text.push_str(lines[i]);
                jsdoc_text.push('\n');
                if lines[i].contains("*/") {
                    break;
                }
                i += 1;
            }

            // 解析 JSDoc
            let parsed = parse_jsdoc(&jsdoc_text);

            // 找到注释后的第一个非注释、非空行
            let mut j = i + 1;
            while j < lines.len()
                && (lines[j].trim().is_empty()
                    || lines[j].trim().starts_with("//")
                    || (lines[j].trim().starts_with("/*") && !lines[j].trim().starts_with("/**")))
            {
                j += 1;
            }
            if j < lines.len() {
                let code = lines[j].trim();
                // 尝试提取变量名（处理 @type）
                if let Some(var_name) = extract_var_name(code)
                    && let Some(ref ty) = parsed.type_name
                {
                    type_annotations.insert(var_name, ty.clone());
                }
                // 尝试提取函数名（处理 @returns 和 @param）
                // 先尝试 function 声明
                let fn_name_opt = if let Some(fn_name) = extract_fn_name(code) {
                    Some(fn_name)
                } else {
                    // 再尝试变量赋值函数（const foo = function() {} 或 const foo = () => {}）
                    extract_var_name(code)
                };
                if let Some(ref fn_name) = fn_name_opt {
                    // 处理 @returns
                    if let Some(ref ty) = parsed.return_type_name {
                        return_types.insert(fn_name.clone(), ty.clone());
                    }
                    // 处理 @param
                    if !parsed.param_types.is_empty() {
                        param_types.insert(fn_name.clone(), parsed.param_types.clone());
                    }
                }
            }

            // 收集 typedefs
            if let Some(td) = parsed.typedef {
                typedefs.insert(td.name.clone(), td);
            }
        }
        i += 1;
    }

    (typedefs, type_annotations, return_types, param_types)
}

/// 从行中提取变量名（const/let/var 声明）
/// 处理：const x = ... / let x = ... / var x = ...
/// 也处理：export const x = ...
fn extract_var_name(code: &str) -> Option<String> {
    let s = code.trim();
    // 去掉 export 关键字
    let s = if let Some(rest) = s.strip_prefix("export") {
        rest.trim_start()
    } else {
        s
    };

    let after_kw = if let Some(rest) = s.strip_prefix("const") {
        rest.trim_start()
    } else if let Some(rest) = s.strip_prefix("let") {
        rest.trim_start()
    } else if let Some(rest) = s.strip_prefix("var") {
        rest.trim_start()
    } else {
        return None;
    };

    // 读取标识符，直到 = ; , 或换行
    let end = after_kw
        .find(&['=', ';', ','][..])
        .unwrap_or(after_kw.len());
    let name = after_kw[..end].trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

/// 从行中提取函数名（function 声明）
/// 处理：function foo(...) / export function foo(...)
fn extract_fn_name(code: &str) -> Option<String> {
    let s = code.trim();
    let s = if let Some(rest) = s.strip_prefix("export") {
        rest.trim_start()
    } else {
        s
    };
    // Handle "async function" declarations
    let s = if let Some(rest) = s.strip_prefix("async") {
        rest.trim_start()
    } else {
        s
    };
    if let Some(rest) = s.strip_prefix("function") {
        let rest = rest.trim_start();
        let end = rest
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .unwrap_or(rest.len());
        let name = rest[..end].trim();
        if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        }
    } else {
        None
    }
}

/// 从 @typedef 行提取类型名
/// 输入："{Object} User" 或 "User" 或 "{Object} User - description"
fn extract_typedef_name(s: &str) -> String {
    let s = s.trim();
    // 去掉 {...} 包装
    let without_brace = if s.starts_with('{') {
        let end = s.find('}').unwrap_or(s.len());
        &s[end + 1..]
    } else {
        s
    };
    // 取第一个非空 token 作为类型名
    without_brace
        .split(|c: char| c.is_whitespace() || c == '-')
        .find(|s| !s.is_empty())
        .unwrap_or("")
        .to_string()
}

/// 从 {...} 中提取类型名
/// For @type {{name: string, age: number}}, returns "{name: string, age: number}"
/// (preserves the inner braces for anonymous object types)
fn extract_braced_type(s: &str) -> String {
    let s = s.trim();
    if s.starts_with('{') {
        // Find matching closing brace (handle nested braces)
        let mut depth = 0;
        let mut end = 0;
        for (i, c) in s.chars().enumerate() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = i;
                        break;
                    }
                }
                _ => {}
            }
        }
        // Check if this is an anonymous object type (contains ":")
        let inner = &s[1..end];
        if inner.contains(':') {
            // Anonymous object type: return with braces
            if inner.trim().starts_with('{') {
                // Double-brace syntax: {{name: string}} → {name: string}
                inner.trim().to_string()
            } else {
                // Single-brace syntax: {name: string} → {name: string}
                s[..end + 1].to_string()
            }
        } else {
            // Named type: return without braces
            inner.trim().to_string()
        }
    } else {
        s.to_string()
    }
}

/// 解析 @property 行
/// 格式：@property {type} fieldName - description
/// 或：@property {type} [fieldName] - description （可选属性）
/// 或：@property fieldName {type}
fn parse_property(s: &str) -> TypedefField {
    let rest = s
        .strip_prefix("@property")
        .or_else(|| s.strip_prefix("@prop"))
        .unwrap_or("")
        .trim();

    // 尝试格式：{type} name 或 {type} [name]
    if rest.starts_with('{') {
        let brace_end = rest.find('}').unwrap_or(rest.len());
        let ty = rest[1..brace_end].trim().to_string();
        let after_brace = rest[brace_end + 1..].trim();

        // 检查是否为可选属性 [name]
        let (name, optional) = if after_brace.starts_with('[') {
            let end = after_brace.find(']').unwrap_or(after_brace.len());
            let name = after_brace[1..end].trim().to_string();
            (name, true)
        } else {
            let name = after_brace
                .split(|c: char| c.is_whitespace() || c == '-')
                .next()
                .unwrap_or("")
                .to_string();
            (name, false)
        };

        return TypedefField { name, ty, optional };
    }

    // 尝试格式：name {type} 或 [name] {type}
    let (name_part, optional) = if rest.starts_with('[') {
        let end = rest.find(']').unwrap_or(rest.len());
        let name = rest[1..end].trim().to_string();
        (name, true)
    } else {
        let parts: Vec<&str> = rest.splitn(2, '{').collect();
        if parts.len() == 2 {
            (parts[0].trim().to_string(), false)
        } else {
            (rest.to_string(), false)
        }
    };

    if let Some(brace_pos) = name_part.find('{') {
        let name = name_part[..brace_pos].trim().to_string();
        let ty = name_part[brace_pos + 1..]
            .strip_suffix('}')
            .unwrap_or(&name_part[brace_pos + 1..])
            .trim()
            .to_string();
        return TypedefField { name, ty, optional };
    }

    // 只有 name，无类型
    TypedefField {
        name: name_part,
        ty: "string".to_string(),
        optional: false,
    }
}

/// 将 JSDoc 类型字符串转为 Zig 类型字符串
/// "string"  → "[]const u8"
/// "number"  → "i64"
/// "boolean" → "bool"
/// "string[]" → "[]const []const u8"
/// "number[]" → "[]const i64"
/// "User[]"  → "[]const User"  (自定义类型的数组)
pub fn jsdoc_type_to_zig(jsdoc_ty: &str, typedefs: &HashMap<String, TypedefDef>) -> String {
    let trimmed = jsdoc_ty.trim();

    // 处理数组类型（以 [] 结尾）
    if let Some(stripped) = trimmed.strip_suffix("[]") {
        let base_type = stripped.trim();

        // 检查 base_type 是否是自定义类型
        if typedefs.contains_key(base_type) {
            // 自定义类型的数组：User[] → []const User
            return format!("[]const {}", base_type);
        }

        // 基本类型的数组
        return match base_type {
            "string" => "[]const []const u8".to_string(),
            "number" => "[]const i64".to_string(),
            "boolean" => "[]const bool".to_string(),
            _ => format!("[]const {}", base_type), // 未知类型，按自定义类型处理
        };
    }

    // 非数组类型
    match trimmed {
        "string" | "str" => "[]const u8".to_string(),
        "number" => "i64".to_string(),
        "boolean" => "bool".to_string(),
        // Built-in runtime types
        "Symbol" => "JsSymbol".to_string(),
        "Date" => "js_date.JsDate".to_string(),
        // 自定义类型名（@typedef 定义的），直接返回
        _ => jsdoc_ty.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_typedef() {
        let jsdoc = r#"
/**
 * @typedef {Object} User
 * @property {string} name
 * @property {number} age
 * @property {boolean} active
 * @property {string} [email]  ← 可选属性
 */
"#;
        let parsed = parse_jsdoc(jsdoc);
        let user = parsed.typedef.expect("test: expected @typedef to be parsed");
        assert_eq!(user.name, "User");
        assert_eq!(user.fields.len(), 4);
        assert_eq!(user.fields[0].name, "name");
        assert_eq!(user.fields[0].ty, "string");
        assert!(!user.fields[0].optional);
        assert_eq!(user.fields[1].name, "age");
        assert_eq!(user.fields[1].ty, "number");
        assert_eq!(user.fields[3].name, "email");
        assert!(user.fields[3].optional);
    }

    #[test]
    fn test_parse_type_annotation() {
        let jsdoc = r#"
/**
 * @type {User}
 */
"#;
        let parsed = parse_jsdoc(jsdoc);
        assert_eq!(parsed.type_name, Some("User".to_string()));
    }

    #[test]
    fn test_parse_returns() {
        let jsdoc = r#"
/**
 * @returns {string}
 */
"#;
        let parsed = parse_jsdoc(jsdoc);
        assert_eq!(parsed.return_type_name, Some("string".to_string()));
    }

    #[test]
    fn test_parse_returns_single_line() {
        // 单行 JSDoc：/** @returns {f64} */
        let parsed = parse_jsdoc("/** @returns {f64} */");
        assert_eq!(parsed.return_type_name, Some("f64".to_string()));
    }

    #[test]
    fn test_extract_all_jsdoc_single_line() {
        // 单行 JSDoc 必须能关联到紧随其后的函数
        let source = r#"
/** @returns {f64} */
export function mathFloor(x) { return Math.floor(x); }
"#;
        let (_typedefs, _type_annots, return_types, _param_types) = extract_all_jsdoc(source);
        assert_eq!(return_types.get("mathFloor"), Some(&"f64".to_string()));
    }

    #[test]
    fn test_jsdoc_type_to_zig() {
        let empty_typedefs = HashMap::new();
        assert_eq!(jsdoc_type_to_zig("string", &empty_typedefs), "[]const u8");
        assert_eq!(jsdoc_type_to_zig("number", &empty_typedefs), "i64");
        assert_eq!(jsdoc_type_to_zig("boolean", &empty_typedefs), "bool");
        assert_eq!(jsdoc_type_to_zig("User", &empty_typedefs), "User");
    }

    #[test]
    fn test_extract_all_jsdoc() {
        let source = r#"
/**
 * @typedef {Object} User
 * @property {string} name
 * @property {number} age
 */

/**
 * @type {User}
 */
const user = JSON.parse(data);

/**
 * @returns {string}
 */
function getName(u) {
    return u.name;
}
"#;
        let (typedefs, type_annots, return_types, param_types) = extract_all_jsdoc(source);
        assert_eq!(typedefs.len(), 1);
        assert!(typedefs.contains_key("User"));
        assert_eq!(type_annots.len(), 1);
        assert_eq!(type_annots["user"], "User");
        assert_eq!(return_types.len(), 1);
        assert_eq!(return_types["getName"], "string");
        assert_eq!(param_types.len(), 0); // No @param in this test
    }

    #[test]
    fn test_parse_param() {
        let jsdoc = r#"
/**
 * @param {string} name
 * @param {number} age
 * @param {boolean} active
 * @returns {string}
 */
"#;
        let parsed = parse_jsdoc(jsdoc);
        assert_eq!(parsed.param_types.len(), 3);
        assert_eq!(
            parsed.param_types[0],
            ("name".to_string(), "string".to_string())
        );
        assert_eq!(
            parsed.param_types[1],
            ("age".to_string(), "number".to_string())
        );
        assert_eq!(
            parsed.param_types[2],
            ("active".to_string(), "boolean".to_string())
        );
        assert_eq!(parsed.return_type_name, Some("string".to_string()));
    }

    #[test]
    fn test_extract_all_jsdoc_with_param() {
        let source = r#"
/**
 * @param {string} name
 * @param {number} age
 * @returns {string}
 */
function greet(name, age) {
    return "Hello " + name + ", age " + age;
}
"#;
        let (typedefs, type_annots, return_types, param_types) = extract_all_jsdoc(source);
        assert_eq!(typedefs.len(), 0);
        assert_eq!(type_annots.len(), 0);
        assert_eq!(return_types.len(), 1);
        assert_eq!(return_types["greet"], "string");
        assert_eq!(param_types.len(), 1);
        assert_eq!(param_types["greet"].len(), 2);
        assert_eq!(
            param_types["greet"][0],
            ("name".to_string(), "string".to_string())
        );
        assert_eq!(
            param_types["greet"][1],
            ("age".to_string(), "number".to_string())
        );
    }

    #[test]
    fn test_parse_anonymous_object_type() {
        let jsdoc = r#"
/**
 * @type {{name: string, age: number}}
 */
"#;
        let parsed = parse_jsdoc(jsdoc);
        assert_eq!(
            parsed.type_name,
            Some("{name: string, age: number}".to_string())
        );
    }

    #[test]
    fn test_parse_returns_anonymous_object() {
        let jsdoc = r#"
/**
 * @returns {{name: string, age: number}}
 */
"#;
        let parsed = parse_jsdoc(jsdoc);
        assert_eq!(
            parsed.return_type_name,
            Some("{name: string, age: number}".to_string())
        );
    }

    #[test]
    fn test_parse_nested_anonymous_object() {
        let jsdoc = r#"
/**
 * @type {{name: string, address: {city: string, zip: number}}}
 */
"#;
        let parsed = parse_jsdoc(jsdoc);
        assert_eq!(
            parsed.type_name,
            Some("{name: string, address: {city: string, zip: number}}".to_string())
        );
    }

    #[test]
    fn test_parse_anonymous_object_array() {
        // Array of anonymous objects: @type {{name: string, age: number}[]}
        // The [] is inside the outer JSDoc braces, so extract_braced_type
        // includes it with the inner anonymous object.
        let jsdoc = r#"
/**
 * @type {{name: string, age: number}[]}
 */
"#;
        let parsed = parse_jsdoc(jsdoc);
        assert_eq!(
            parsed.type_name,
            Some("{name: string, age: number}[]".to_string())
        );
    }
}
