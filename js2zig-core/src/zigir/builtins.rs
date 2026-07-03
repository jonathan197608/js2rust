// zigir/builtins.rs
// Builtin module classification for IR.

/// Runtime module that a builtin method belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinModule {
    JsArray,
    JsString,
    JsDate,
    JsJson,
    JsObject,
    JsNumber,
    JsSymbol,
    JsConsole,
    JsMath,
    JsRegExp,
    JsTypedArray,
    JsUri,
    JsBigInt,
    JsCollections,
}

impl BuiltinModule {
    /// The Zig runtime module path prefix (e.g. "js_array").
    pub fn module_path(self) -> &'static str {
        match self {
            BuiltinModule::JsArray => "js_array",
            BuiltinModule::JsString => "js_string",
            BuiltinModule::JsDate => "js_date",
            BuiltinModule::JsJson => "js_json",
            BuiltinModule::JsObject => "js_object",
            BuiltinModule::JsNumber => "js_number",
            BuiltinModule::JsSymbol => "js_symbol",
            BuiltinModule::JsConsole => "js_console",
            BuiltinModule::JsMath => "std.math",
            BuiltinModule::JsRegExp => "js_regexp",
            BuiltinModule::JsTypedArray => "js_typedarray",
            BuiltinModule::JsUri => "js_uri",
            BuiltinModule::JsBigInt => "js_bigint",
            BuiltinModule::JsCollections => "js_collections",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_modules() {
        let modules = [
            BuiltinModule::JsArray,
            BuiltinModule::JsString,
            BuiltinModule::JsDate,
            BuiltinModule::JsJson,
            BuiltinModule::JsObject,
            BuiltinModule::JsNumber,
            BuiltinModule::JsSymbol,
            BuiltinModule::JsConsole,
            BuiltinModule::JsMath,
            BuiltinModule::JsRegExp,
            BuiltinModule::JsTypedArray,
            BuiltinModule::JsUri,
            BuiltinModule::JsBigInt,
            BuiltinModule::JsCollections,
        ];
        assert_eq!(modules.len(), 14);
        // Verify each module path is non-empty
        for m in &modules {
            assert!(!m.module_path().is_empty());
        }
    }
}
