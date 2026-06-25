// native_builtins.rs — moved from native_proto/builtins.rs
// Built-in object methods (Math, Array, String, etc.)
//
// This module only defines the BuiltinCall enum and detection function.
// The emission logic is in codegen.rs (since it needs to call private methods).

use crate::native_proto::ZigType;

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
    MathHypot,  // Math.hypot(...) — 不支持，报编译错误

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
    ArraySplice,   // arr.splice(start, deleteCount, ...items)

    // Array methods (with closure)
    ArrayForEach, // arr.forEach(fn)
    ArrayMap,     // arr.map(fn)
    ArrayFilter,  // arr.filter(fn)
    ArrayReduce,  // arr.reduce(fn, init)
    ArraySome,    // arr.some(fn)
    ArrayEvery,   // arr.every(fn)
    ArrayFlat,    // arr.flat()
    ArrayFlatMap, // arr.flatMap(fn)

    // TypedArray methods (.get/.set routed through MapGet/MapSet in codegen,
    // .slice routed through ArraySlice + typedarray_vars check)
    TypedArraySubarray,   // arr.subarray(start, end)
    TypedArrayCopyWithin, // arr.copyWithin(target, start, end)
    TypedArrayFill,       // arr.fill(val, start, end)

    // String methods
    StringIndexOf,    // str.indexOf(search)
    StringIncludes,   // str.includes(search)
    StringStartsWith, // str.startsWith(prefix)
    StringEndsWith,   // str.endsWith(suffix)
    StringTrim,       // str.trim()
    StringSplit,      // str.split(sep)
    StringPadStart,   // str.padStart(len, pad)
    StringPadEnd,     // str.padEnd(len, pad)

    // Map methods (called on local Map variables)
    MapSet,    // map.set(key, value)
    MapGet,    // map.get(key)
    MapHas,    // map.has(key) or set.has(value)
    MapDelete, // map.delete(key) or set.delete(value)

    // Set methods (called on local Set variables)
    SetAdd, // set.add(value)

    // Date methods (static)
    DateNow,   // Date.now() → i64
    DateParse, // Date.parse(str) → i64
    DateUTC,   // Date.UTC(y, m, d) → i64

    // Date methods (instance — called on an i64 millis value)
    DateGetTime,     // date.getTime()
    DateGetFullYear, // date.getFullYear()
    DateGetMonth,    // date.getMonth()
    DateGetDate,     // date.getDate()
    DateGetDay,      // date.getDay()
    DateGetHours,    // date.getHours()
    DateGetMinutes,  // date.getMinutes()
    DateGetSeconds,  // date.getSeconds()

    // Object methods (static)
    ObjectKeys,    // Object.keys(obj)
    ObjectValues,  // Object.values(obj)
    ObjectEntries, // Object.entries(obj)
    ObjectAssign,  // Object.assign(target, source)
    ObjectFreeze,  // Object.freeze(obj)

    // Global functions
    ParseInt, // parseInt(s)

    // JSON methods
    JsonStringify, // JSON.stringify(value, replacer?, space?)
    JsonParse,     // JSON.parse(text, reviver?)
}

/// Check if a call expression is a built-in object call
/// Returns Some(BuiltinCall) if it is, None otherwise
pub fn detect_builtin_call(ce: &oxc_ast::ast::CallExpression) -> Option<BuiltinCall> {
    use oxc_ast::ast::*;

    // Global function calls (plain identifier callee)
    if let Expression::Identifier(id) = &ce.callee {
        match id.name.as_str() {
            "parseInt" => return Some(BuiltinCall::ParseInt),
            _ => return None,
        }
    }

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
                "hypot" => return Some(BuiltinCall::MathHypot),
                _ => return None,
            }
        }

        // Check if object is "Date" (for Date static methods)
        if let Expression::Identifier(id) = obj_expr
            && id.name.as_str() == "Date"
        {
            match method_name {
                "now" => return Some(BuiltinCall::DateNow),
                "parse" => return Some(BuiltinCall::DateParse),
                "UTC" => return Some(BuiltinCall::DateUTC),
                _ => return None,
            }
        }

        // Check if object is "Object" (for Object static methods)
        if let Expression::Identifier(id) = obj_expr
            && id.name.as_str() == "Object"
        {
            match method_name {
                "keys" => return Some(BuiltinCall::ObjectKeys),
                "values" => return Some(BuiltinCall::ObjectValues),
                "entries" => return Some(BuiltinCall::ObjectEntries),
                "assign" => return Some(BuiltinCall::ObjectAssign),
                "freeze" => return Some(BuiltinCall::ObjectFreeze),
                _ => return None,
            }
        }

        // Check if object is "JSON" (for JSON methods)
        if let Expression::Identifier(id) = obj_expr
            && id.name.as_str() == "JSON"
        {
            match method_name {
                "stringify" => return Some(BuiltinCall::JsonStringify),
                "parse" => return Some(BuiltinCall::JsonParse),
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
            "padStart" => Some(BuiltinCall::StringPadStart),
            "padEnd" => Some(BuiltinCall::StringPadEnd),

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

            "pop" => Some(BuiltinCall::ArrayPop),
            "shift" => Some(BuiltinCall::ArrayShift),
            "unshift" => Some(BuiltinCall::ArrayUnshift),
            "reverse" => Some(BuiltinCall::ArrayReverse),
            "sort" => Some(BuiltinCall::ArraySort),
            "join" => Some(BuiltinCall::ArrayJoin),
            "slice" => Some(BuiltinCall::ArraySlice), // also handled as TypedArray in emit_builtin_call
            "splice" => Some(BuiltinCall::ArraySplice),
            "forEach" => Some(BuiltinCall::ArrayForEach),
            "map" => Some(BuiltinCall::ArrayMap),
            "filter" => Some(BuiltinCall::ArrayFilter),
            "reduce" => Some(BuiltinCall::ArrayReduce),
            "some" => Some(BuiltinCall::ArraySome),
            "every" => Some(BuiltinCall::ArrayEvery),
            "flat" => Some(BuiltinCall::ArrayFlat),
            "flatMap" => Some(BuiltinCall::ArrayFlatMap),

            // TypedArray-specific methods (non-overlapping with Array)
            "subarray" => Some(BuiltinCall::TypedArraySubarray),
            "copyWithin" => Some(BuiltinCall::TypedArrayCopyWithin),
            "fill" => Some(BuiltinCall::TypedArrayFill),

            // Date instance methods (called on an i64 millis value)
            "getTime" => Some(BuiltinCall::DateGetTime),
            "getFullYear" => Some(BuiltinCall::DateGetFullYear),
            "getMonth" => Some(BuiltinCall::DateGetMonth),
            "getDate" => Some(BuiltinCall::DateGetDate),
            "getDay" => Some(BuiltinCall::DateGetDay),
            "getHours" => Some(BuiltinCall::DateGetHours),
            "getMinutes" => Some(BuiltinCall::DateGetMinutes),
            "getSeconds" => Some(BuiltinCall::DateGetSeconds),

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

/// Return the Zig type of a built-in call result, if it can be statically determined.
/// Returns None for methods whose return type depends on arguments (e.g., Math.max/min).
pub fn builtin_return_type(builtin: &BuiltinCall) -> Option<ZigType> {
    match builtin {
        // Math methods — all return f64
        BuiltinCall::MathAbs
        | BuiltinCall::MathFloor
        | BuiltinCall::MathCeil
        | BuiltinCall::MathRound
        | BuiltinCall::MathSqrt
        | BuiltinCall::MathRandom
        | BuiltinCall::MathPow => Some(ZigType::F64),

        // Math max/min — depends on args, can't statically determine
        BuiltinCall::MathMax | BuiltinCall::MathMin | BuiltinCall::MathHypot => None,

        // String methods
        BuiltinCall::StringIndexOf => Some(ZigType::I64),
        BuiltinCall::StringIncludes
        | BuiltinCall::StringStartsWith
        | BuiltinCall::StringEndsWith => Some(ZigType::Bool),
        BuiltinCall::StringTrim | BuiltinCall::StringSplit => Some(ZigType::Str),

        // Map methods
        BuiltinCall::MapGet => Some(ZigType::Anytype), // Conservative
        BuiltinCall::MapHas => Some(ZigType::Bool),

        // Date static methods
        BuiltinCall::DateNow | BuiltinCall::DateParse | BuiltinCall::DateUTC => Some(ZigType::I64),

        // Date instance methods
        BuiltinCall::DateGetTime
        | BuiltinCall::DateGetFullYear
        | BuiltinCall::DateGetMonth
        | BuiltinCall::DateGetDate
        | BuiltinCall::DateGetDay
        | BuiltinCall::DateGetHours
        | BuiltinCall::DateGetMinutes
        | BuiltinCall::DateGetSeconds => Some(ZigType::I64),

        // Object methods
        BuiltinCall::ObjectKeys | BuiltinCall::ObjectValues | BuiltinCall::ObjectEntries => {
            Some(ZigType::ArrayList(Box::new(ZigType::Str)))
        }

        // Global functions
        BuiltinCall::ParseInt => Some(ZigType::I64),

        // JSON methods
        BuiltinCall::JsonStringify => Some(ZigType::Str), // Returns JSON string
        BuiltinCall::JsonParse => Some(ZigType::JsAny),   // Returns dynamic JSON value

        // Methods that return void or complex types — can't infer
        _ => None,
    }
}
