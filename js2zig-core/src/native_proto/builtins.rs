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
    MathRandom,   // Math.random()
    MathPow,      // Math.pow(base, exp)
    MathMax,      // Math.max(...args)
    MathMin,      // Math.min(...args)
    
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
        // Get object expression
        let obj_expr = &mem.object;
        
        // Get method name
        let method_name = mem.property.name.as_str();
        
        // Check if object is "Math" (for Math methods)
        if let Expression::Identifier(id) = obj_expr {
            if id.name.as_str() == "Math" {
                // Math methods
                match method_name {
                    "abs" => return Some(BuiltinCall::MathAbs),
                    "floor" => return Some(BuiltinCall::MathFloor),
                    "ceil" => return Some(BuiltinCall::MathCeil),
                    "round" => return Some(BuiltinCall::MathRound),
                    "sqrt" => return Some(BuiltinCall::MathSqrt),
                    "random" => return Some(BuiltinCall::MathRandom),
                    "pow" => return Some(BuiltinCall::MathPow),
                    "max" => return Some(BuiltinCall::MathMax),
                    "min" => return Some(BuiltinCall::MathMin),
                    _ => return None,
                }
            }
        }
        
        // Check if object is a string literal (for String methods)
        let is_string = matches!(obj_expr, Expression::StringLiteral(_));
        
        // Detect methods based on object type and method name
        match method_name {
            // String-specific methods (always String methods)
            "startsWith" => return Some(BuiltinCall::StringStartsWith),
            "endsWith" => return Some(BuiltinCall::StringEndsWith),
            "trim" => return Some(BuiltinCall::StringTrim),
            "split" => return Some(BuiltinCall::StringSplit),
            
            // Methods that exist on both String and Array
            "indexOf" => {
                if is_string {
                    return Some(BuiltinCall::StringIndexOf);
                } else {
                    return Some(BuiltinCall::ArrayIndexOf);
                }
            }
            "includes" => {
                if is_string {
                    return Some(BuiltinCall::StringIncludes);
                } else {
                    return Some(BuiltinCall::ArrayIncludes);
                }
            }
            
            // Array-specific methods
            "pop" => return Some(BuiltinCall::ArrayPop),
            "join" => return Some(BuiltinCall::ArrayJoin),
            "slice" => return Some(BuiltinCall::ArraySlice),
            
            _ => return None,
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
        BuiltinCall::MathRandom => ZigType::F64,
        BuiltinCall::MathPow => ZigType::F64,
        BuiltinCall::MathMax => ZigType::F64,
        BuiltinCall::MathMin => ZigType::F64,
        
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
