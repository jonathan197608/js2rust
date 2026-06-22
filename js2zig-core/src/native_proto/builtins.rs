// native_proto/builtins.rs
// Built-in object methods (Math, Array, String, etc.)
//
// This module only defines the BuiltinCall enum and detection function.
// The emission logic is in codegen.rs (since it needs to call private methods).

/// Built-in call type
#[derive(Debug, Clone, PartialEq)]
pub enum BuiltinCall {
    // Math methods
    MathAbs,    // Math.abs(x)
    MathFloor,  // Math.floor(x)
    MathCeil,   // Math.ceil(x)
    MathRound,  // Math.round(x)
    MathSqrt,   // Math.sqrt(x)
    MathRandom, // Math.random()
    MathPow,    // Math.pow(base, exp)
    MathMax,    // Math.max(...args)
    MathMin,    // Math.min(...args)

    // Array methods (non-closure)
    ArrayPop,      // arr.pop()
    ArrayShift,    // arr.shift()
    ArrayUnshift,  // arr.unshift(x)
    ArrayReverse,  // arr.reverse()
    ArraySort,     // arr.sort()
    ArrayIndexOf,  // arr.indexOf(x)
    ArrayIncludes, // arr.includes(x)
    ArrayJoin,     // arr.join(sep)
    ArraySlice,    // arr.slice(start, end)

    // Array methods (with closure)
    ArrayForEach, // arr.forEach(fn)
    ArrayMap,     // arr.map(fn)
    ArrayFilter,  // arr.filter(fn)
    ArrayReduce,  // arr.reduce(fn, init)
    ArraySome,    // arr.some(fn)
    ArrayEvery,   // arr.every(fn)

    // String methods
    StringIndexOf,    // str.indexOf(search)
    StringIncludes,   // str.includes(search)
    StringStartsWith, // str.startsWith(prefix)
    StringEndsWith,   // str.endsWith(suffix)
    StringTrim,       // str.trim()
    StringSplit,      // str.split(sep)

    // Map methods (called on local Map variables)
    MapSet,    // map.set(key, value)
    MapGet,    // map.get(key)
    MapHas,    // map.has(key) or set.has(value)
    MapDelete, // map.delete(key) or set.delete(value)

    // Set methods (called on local Set variables)
    SetAdd, // set.add(value)
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
        if let Expression::Identifier(id) = obj_expr
            && id.name.as_str() == "Math"
        {
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

        // Check if object is a string literal (for String methods)
        let is_string = matches!(obj_expr, Expression::StringLiteral(_));

        // Detect methods based on object type and method name
        match method_name {
            // String-specific methods (always String methods)
            "startsWith" => Some(BuiltinCall::StringStartsWith),
            "endsWith" => Some(BuiltinCall::StringEndsWith),
            "trim" => Some(BuiltinCall::StringTrim),
            "split" => Some(BuiltinCall::StringSplit),

            // Methods that exist on both String and Array
            "indexOf" => {
                if is_string {
                    Some(BuiltinCall::StringIndexOf)
                } else {
                    Some(BuiltinCall::ArrayIndexOf)
                }
            }
            "includes" => {
                if is_string {
                    Some(BuiltinCall::StringIncludes)
                } else {
                    Some(BuiltinCall::ArrayIncludes)
                }
            }

            // Array-specific methods
            "pop" => Some(BuiltinCall::ArrayPop),
            "shift" => Some(BuiltinCall::ArrayShift),
            "unshift" => Some(BuiltinCall::ArrayUnshift),
            "reverse" => Some(BuiltinCall::ArrayReverse),
            "sort" => Some(BuiltinCall::ArraySort),
            "join" => Some(BuiltinCall::ArrayJoin),
            "slice" => Some(BuiltinCall::ArraySlice),
            "forEach" => Some(BuiltinCall::ArrayForEach),
            "map" => Some(BuiltinCall::ArrayMap),
            "filter" => Some(BuiltinCall::ArrayFilter),
            "reduce" => Some(BuiltinCall::ArrayReduce),
            "some" => Some(BuiltinCall::ArraySome),
            "every" => Some(BuiltinCall::ArrayEvery),

            // Map methods (called on local Map variables)
            "set" => Some(BuiltinCall::MapSet),
            "get" => Some(BuiltinCall::MapGet),
            "has" => {
                // Could be Map.has() or Set.has()
                // Default to Map.has(), will be resolved in codegen
                Some(BuiltinCall::MapHas)
            }
            "delete" => {
                // Could be Map.delete() or Set.delete()
                // Default to Map.delete(), will be resolved in codegen
                Some(BuiltinCall::MapDelete)
            }

            // Set methods (called on local Set variables)
            "add" => Some(BuiltinCall::SetAdd),

            _ => None,
        }
    } else {
        None
    }
}
