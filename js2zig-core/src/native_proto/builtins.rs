// native_proto/builtins.rs
// Built-in object methods (Math, Array, String, etc.)
//
// This module only defines the BuiltinCall enum and detection function.
// The emission logic is in codegen.rs (since it needs to call private methods).

/// Built-in call type
#[derive(Debug, Clone, PartialEq)]
pub enum BuiltinCall {
    // Math methods
    MathAbs,      // Math.abs(x)
    MathFloor,    // Math.floor(x)
    MathCeil,     // Math.ceil(x)
    MathRound,    // Math.round(x)
    MathSqrt,     // Math.sqrt(x)
    
    // Array methods (non-closure)
    ArrayPop,     // arr.pop()
    ArrayIndexOf,  // arr.indexOf(x)
    ArrayIncludes, // arr.includes(x)
    ArrayJoin,     // arr.join(sep)
    ArraySlice,    // arr.slice(start, end)
    
    // String methods
    StringIndexOf,    // str.indexOf(search)
    StringIncludes,   // str.includes(search)
    StringStartsWith, // str.startsWith(prefix)
    StringEndsWith,   // str.endsWith(suffix)
    StringTrim,       // str.trim()
    StringSplit,      // str.split(sep)
}

/// Check if a call expression is a built-in object call
/// Returns Some(BuiltinCall) if it is, None otherwise
pub fn detect_builtin_call(ce: &oxc_ast::ast::CallExpression) -> Option<BuiltinCall> {
    use oxc_ast::ast::*;
    
    // Check if callee is a StaticMemberExpression (obj.method())
    if let Expression::StaticMemberExpression(mem) = &ce.callee {
        // Get object name
        let obj_name = if let Expression::Identifier(id) = &mem.object {
            id.name.as_str()
        } else {
            return None;
        };
        
        // Get method name
        let method_name = mem.property.name.as_str();
        
        // Match built-in calls
        match (obj_name, method_name) {
            ("Math", "abs") => Some(BuiltinCall::MathAbs),
            ("Math", "floor") => Some(BuiltinCall::MathFloor),
            ("Math", "ceil") => Some(BuiltinCall::MathCeil),
            ("Math", "round") => Some(BuiltinCall::MathRound),
            ("Math", "sqrt") => Some(BuiltinCall::MathSqrt),
            
            // Array methods (object is a variable, not "Math")
            (_, "pop") => Some(BuiltinCall::ArrayPop),
            (_, "indexOf") => Some(BuiltinCall::ArrayIndexOf),
            (_, "includes") => Some(BuiltinCall::ArrayIncludes),
            (_, "join") => Some(BuiltinCall::ArrayJoin),
            (_, "slice") => Some(BuiltinCall::ArraySlice),
            
            // String methods
            (_, "startsWith") => Some(BuiltinCall::StringStartsWith),
            (_, "endsWith") => Some(BuiltinCall::StringEndsWith),
            (_, "trim") => Some(BuiltinCall::StringTrim),
            (_, "split") => Some(BuiltinCall::StringSplit),
            
            _ => None,
        }
    } else {
        None
    }
}

/// Get the return type of a built-in call
pub fn builtin_return_type(builtin: &BuiltinCall) -> crate::native_proto::ZigType {
    use crate::native_proto::ZigType;
    
    match builtin {
        BuiltinCall::MathAbs => ZigType::F64,
        BuiltinCall::MathFloor => ZigType::F64,
        BuiltinCall::MathCeil => ZigType::F64,
        BuiltinCall::MathRound => ZigType::F64,
        BuiltinCall::MathSqrt => ZigType::F64,
        
        BuiltinCall::ArrayPop => ZigType::I64, // TODO: should be ?T
        BuiltinCall::ArrayIndexOf => ZigType::I64, // TODO: should be ?usize
        BuiltinCall::ArrayIncludes => ZigType::Bool,
        BuiltinCall::ArrayJoin => ZigType::Str,
        BuiltinCall::ArraySlice => ZigType::Str, // TODO: should return new array
        
        BuiltinCall::StringIndexOf => ZigType::I64, // TODO: should be ?usize
        BuiltinCall::StringIncludes => ZigType::Bool,
        BuiltinCall::StringStartsWith => ZigType::Bool,
        BuiltinCall::StringEndsWith => ZigType::Bool,
        BuiltinCall::StringTrim => ZigType::Str,
        BuiltinCall::StringSplit => ZigType::Str, // TODO: should return []const []const u8
    }
}
