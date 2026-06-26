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
    // Math trig
    MathSin,   // Math.sin(x)
    MathCos,   // Math.cos(x)
    MathTan,   // Math.tan(x)
    MathAsin,  // Math.asin(x)
    MathAcos,  // Math.acos(x)
    MathAtan,  // Math.atan(x)
    MathAtan2, // Math.atan2(y, x)
    // Math log / other
    MathLog,   // Math.log(x)
    MathLog10, // Math.log10(x)
    MathLog2,  // Math.log2(x)
    MathExp,   // Math.exp(x)
    MathSign,  // Math.sign(x)
    MathTrunc, // Math.trunc(x)
    MathCbrt,  // Math.cbrt(x)

    // Math methods (extended — Phase 4)
    MathExpm1,  // Math.expm1(x)
    MathSinh,   // Math.sinh(x)
    MathCosh,   // Math.cosh(x)
    MathTanh,   // Math.tanh(x)
    MathAsinh,  // Math.asinh(x)
    MathAcosh,  // Math.acosh(x)
    MathAtanh,  // Math.atanh(x)
    MathClz32,  // Math.clz32(x)
    MathFround, // Math.fround(x)
    MathImul,   // Math.imul(a, b)
    MathLog1p,  // Math.log1p(x)

    // Array methods (non-closure)
    ArrayPop,         // arr.pop()
    ArrayShift,       // arr.shift()
    ArrayUnshift,     // arr.unshift(x)
    ArrayReverse,     // arr.reverse()
    ArraySort,        // arr.sort()
    ArrayIndexOf,     // arr.indexOf(x)
    ArrayIncludes,    // arr.includes(x)
    ArrayJoin,        // arr.join(sep)
    ArraySlice,       // arr.slice(start, end)
    ArraySplice,      // arr.splice(start, deleteCount, ...items)
    ArrayConcat,      // arr.concat(other)
    ArrayAt,          // arr.at(index) — negative index support
    ArrayLastIndexOf, // arr.lastIndexOf(x)
    ArrayCopyWithin,  // arr.copyWithin(target, start, end)

    // Array iterator methods
    ArrayKeys,    // arr.keys() → number[] (iterator of indices)
    ArrayValues,  // arr.values() → any[] (iterator of values)
    ArrayEntries, // arr.entries() → [number, any][] (iterator of [index, value])

    // Array methods (with closure)
    ArrayForEach,       // arr.forEach(fn)
    ArrayMap,           // arr.map(fn)
    ArrayFilter,        // arr.filter(fn)
    ArrayReduce,        // arr.reduce(fn, init)
    ArraySome,          // arr.some(fn)
    ArrayEvery,         // arr.every(fn)
    ArrayFlat,          // arr.flat()
    ArrayFlatMap,       // arr.flatMap(fn)
    ArrayFind,          // arr.find(fn)
    ArrayFindIndex,     // arr.findIndex(fn)
    ArrayFindLast,      // arr.findLast(fn)
    ArrayFindLastIndex, // arr.findLastIndex(fn)
    ArrayReduceRight,   // arr.reduceRight(fn, init)
    ArrayFill,          // arr.fill(val, start, end)

    // Array static methods
    ArrayFrom,    // Array.from(arrayLike[, mapFn[, thisArg]])
    ArrayOf,      // Array.of(...items)
    ArrayIsArray, // Array.isArray(obj)

    // TypedArray methods (.get/.set routed through MapGet/MapSet in codegen,
    // .slice routed through ArraySlice + typedarray_vars check)
    TypedArraySubarray, // arr.subarray(start, end)

    // String methods
    StringIndexOf,     // str.indexOf(search)
    StringIncludes,    // str.includes(search)
    StringStartsWith,  // str.startsWith(prefix)
    StringEndsWith,    // str.endsWith(suffix)
    StringLastIndexOf, // str.lastIndexOf(search)
    StringTrim,        // str.trim()
    StringSplit,       // str.split(sep)
    StringPadStart,    // str.padStart(len, pad)
    StringPadEnd,      // str.padEnd(len, pad)
    StringTrimStart,   // str.trimStart()
    StringTrimEnd,     // str.trimEnd()
    StringMatch,       // str.match(regex) — stub (regex not yet supported)
    StringSearch,      // str.search(regex) — stub (regex not yet supported)

    // Map methods (called on local Map variables)
    MapSet,     // map.set(key, value)
    MapGet,     // map.get(key)
    MapHas,     // map.has(key) or set.has(value)
    MapDelete,  // map.delete(key) or set.delete(value)
    MapKeys,    // map.keys() → string[]
    MapValues,  // map.values() → any[]
    MapEntries, // map.entries() → [string, any][]

    // Set methods (called on local Set variables)
    SetAdd,     // set.add(value)
    SetForEach, // set.forEach(fn)
    SetKeys,    // set.keys() → K[] (same as values)
    SetValues,  // set.values() → K[]
    SetEntries, // set.entries() → [K, K][]

    // Date methods (static)
    DateNow,   // Date.now() → i64
    DateParse, // Date.parse(str) → i64
    DateUTC,   // Date.UTC(y, m, d) → i64

    // Date methods (instance — called on a JsDate struct)
    DateGetTime,           // date.getTime()
    DateGetFullYear,       // date.getFullYear()
    DateGetMonth,          // date.getMonth()
    DateGetDate,           // date.getDate()
    DateGetDay,            // date.getDay()
    DateGetHours,          // date.getHours()
    DateGetMinutes,        // date.getMinutes()
    DateGetSeconds,        // date.getSeconds()
    DateGetMilliseconds,   // date.getMilliseconds()
    DateGetTimezoneOffset, // date.getTimezoneOffset()
    DateToISOString,       // date.toISOString()
    DateToString,          // date.toString()
    DateToDateString,      // date.toDateString()
    DateToTimeString,      // date.toTimeString()
    DateToLocaleString,    // date.toLocaleString()

    // Date methods (UTC getters)
    DateGetUTCFullYear,     // date.getUTCFullYear()
    DateGetUTCMonth,        // date.getUTCMonth()
    DateGetUTCDate,         // date.getUTCDate()
    DateGetUTCDay,          // date.getUTCDay()
    DateGetUTCHours,        // date.getUTCHours()
    DateGetUTCMinutes,      // date.getUTCMinutes()
    DateGetUTCSeconds,      // date.getUTCSeconds()
    DateGetUTCMilliseconds, // date.getUTCMilliseconds()

    // Date methods (toJSON/valueOf)
    DateToJSON,  // date.toJSON()
    DateValueOf, // date.valueOf()

    // Date methods (local setters)
    DateSetFullYear,     // date.setFullYear(year, month?, date?)
    DateSetMonth,        // date.setMonth(month, date?)
    DateSetDate,         // date.setDate(date)
    DateSetHours,        // date.setHours(hours, min?, sec?, ms?)
    DateSetMinutes,      // date.setMinutes(min, sec?, ms?)
    DateSetSeconds,      // date.setSeconds(sec, ms?)
    DateSetMilliseconds, // date.setMilliseconds(ms)

    // Date methods (UTC setters)
    DateSetUTCFullYear,     // date.setUTCFullYear(year, month?, date?)
    DateSetUTCMonth,        // date.setUTCMonth(month, date?)
    DateSetUTCDate,         // date.setUTCDate(date)
    DateSetUTCHours,        // date.setUTCHours(hours, min?, sec?, ms?)
    DateSetUTCMinutes,      // date.setUTCMinutes(min, sec?, ms?)
    DateSetUTCSeconds,      // date.setUTCSeconds(sec, ms?)
    DateSetUTCMilliseconds, // date.setUTCMilliseconds(ms)

    // Object methods (static)
    ObjectKeys,                     // Object.keys(obj)
    ObjectValues,                   // Object.values(obj)
    ObjectEntries,                  // Object.entries(obj)
    ObjectAssign,                   // Object.assign(target, source)
    ObjectFreeze,                   // Object.freeze(obj)
    ObjectSeal,                     // Object.seal(obj) — simplified no-op
    ObjectHasOwn,                   // Object.hasOwn(obj, key)
    ObjectIs,                       // Object.is(a, b) — SameValue comparison
    ObjectGetOwnPropertyNames,      // Object.getOwnPropertyNames(obj)
    ObjectCreate,                   // Object.create(proto)
    ObjectDefineProperty,           // Object.defineProperty(obj, key, desc)
    ObjectGetPrototypeOf,           // Object.getPrototypeOf(obj)
    ObjectDefineProperties,         // Object.defineProperties(obj, props)
    ObjectGetOwnPropertyDescriptor, // Object.getOwnPropertyDescriptor(obj, key)
    ObjectSetPrototypeOf,           // Object.setPrototypeOf(obj, proto) — simplified no-op

    // Global functions
    ParseInt,           // parseInt(s)
    ParseFloat,         // parseFloat(s)
    IsNaN,              // isNaN(v)
    IsFinite,           // isFinite(v)
    EncodeURIComponent, // encodeURIComponent(s)
    DecodeURIComponent, // decodeURIComponent(s)
    EncodeURI,          // encodeURI(s)
    DecodeURI,          // decodeURI(s)

    // Console methods
    ConsoleLog,   // console.log(msg)
    ConsoleError, // console.error(msg)
    ConsoleWarn,  // console.warn(msg)

    // Number static methods
    NumberIsNaN,         // Number.isNaN(v)
    NumberIsFinite,      // Number.isFinite(v)
    NumberIsInteger,     // Number.isInteger(v)
    NumberIsSafeInteger, // Number.isSafeInteger(v)
    NumberParseInt,      // Number.parseInt(s)
    NumberParseFloat,    // Number.parseFloat(s)

    // String static methods
    StringFromCharCode,  // String.fromCharCode(...codes)
    StringFromCodePoint, // String.fromCodePoint(...codePoints)

    // Number instance methods
    NumberToFixed,       // num.toFixed(digits) → str
    NumberToExponential, // num.toExponential(fractionDigits) → str
    NumberToPrecision,   // num.toPrecision(precision) → str

    // String methods (extended)
    StringToUpperCase, // str.toUpperCase()
    StringToLowerCase, // str.toLowerCase()
    StringCharAt,      // str.charAt(idx)
    StringCharCodeAt,  // str.charCodeAt(idx)
    StringCodePointAt, // str.codePointAt(idx) — Unicode code point
    StringConcat,      // str.concat(other)
    StringSlice,       // str.slice(start, end)
    StringReplace,     // str.replace(old, new)
    StringReplaceAll,  // str.replaceAll(old, new)
    StringRepeat,      // str.repeat(n)
    StringSubstring,   // str.substring(start, end)
    StringAt,          // str.at(index) — negative index support

    // String methods (P2 — not yet implemented)
    StringMatchAll,          // str.matchAll(regex) — requires regex support (skip)
    StringLocaleCompare,     // str.localeCompare(other) — simplified (byte-wise comparison)
    StringNormalize,         // str.normalize(form) — stub (pass-through)
    StringToLocaleUpperCase, // str.toLocaleUpperCase() — simplified (uses toUpper)
    StringToLocaleLowerCase, // str.toLocaleLowerCase() — simplified (uses toLower)

    // Map/Set clear (shared variant like MapHas/MapDelete)
    MapClear, // map.clear() or set.clear()

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
            "parseFloat" => return Some(BuiltinCall::ParseFloat),
            "isNaN" => return Some(BuiltinCall::IsNaN),
            "isFinite" => return Some(BuiltinCall::IsFinite),
            "encodeURIComponent" => return Some(BuiltinCall::EncodeURIComponent),
            "decodeURIComponent" => return Some(BuiltinCall::DecodeURIComponent),
            "encodeURI" => return Some(BuiltinCall::EncodeURI),
            "decodeURI" => return Some(BuiltinCall::DecodeURI),
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
                "sin" => return Some(BuiltinCall::MathSin),
                "cos" => return Some(BuiltinCall::MathCos),
                "tan" => return Some(BuiltinCall::MathTan),
                "asin" => return Some(BuiltinCall::MathAsin),
                "acos" => return Some(BuiltinCall::MathAcos),
                "atan" => return Some(BuiltinCall::MathAtan),
                "atan2" => return Some(BuiltinCall::MathAtan2),
                "log" => return Some(BuiltinCall::MathLog),
                "log10" => return Some(BuiltinCall::MathLog10),
                "log2" => return Some(BuiltinCall::MathLog2),
                "exp" => return Some(BuiltinCall::MathExp),
                "sign" => return Some(BuiltinCall::MathSign),
                "trunc" => return Some(BuiltinCall::MathTrunc),
                "cbrt" => return Some(BuiltinCall::MathCbrt),
                // Phase 4 Math methods
                "expm1" => return Some(BuiltinCall::MathExpm1),
                "sinh" => return Some(BuiltinCall::MathSinh),
                "cosh" => return Some(BuiltinCall::MathCosh),
                "tanh" => return Some(BuiltinCall::MathTanh),
                "asinh" => return Some(BuiltinCall::MathAsinh),
                "acosh" => return Some(BuiltinCall::MathAcosh),
                "atanh" => return Some(BuiltinCall::MathAtanh),
                "clz32" => return Some(BuiltinCall::MathClz32),
                "fround" => return Some(BuiltinCall::MathFround),
                "imul" => return Some(BuiltinCall::MathImul),
                "log1p" => return Some(BuiltinCall::MathLog1p),
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
                "seal" => return Some(BuiltinCall::ObjectSeal),
                "hasOwn" => return Some(BuiltinCall::ObjectHasOwn),
                "is" => return Some(BuiltinCall::ObjectIs),
                "getOwnPropertyNames" => return Some(BuiltinCall::ObjectGetOwnPropertyNames),
                "create" => return Some(BuiltinCall::ObjectCreate),
                "defineProperty" => return Some(BuiltinCall::ObjectDefineProperty),
                "getPrototypeOf" => return Some(BuiltinCall::ObjectGetPrototypeOf),
                "defineProperties" => return Some(BuiltinCall::ObjectDefineProperties),
                "getOwnPropertyDescriptor" => {
                    return Some(BuiltinCall::ObjectGetOwnPropertyDescriptor);
                }
                "setPrototypeOf" => return Some(BuiltinCall::ObjectSetPrototypeOf),
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

        // Check if object is "console" (for console methods)
        if let Expression::Identifier(id) = obj_expr
            && id.name.as_str() == "console"
        {
            match method_name {
                "log" => return Some(BuiltinCall::ConsoleLog),
                "error" => return Some(BuiltinCall::ConsoleError),
                "warn" => return Some(BuiltinCall::ConsoleWarn),
                _ => return None,
            }
        }

        // Check if object is "Number" (for Number static methods)
        if let Expression::Identifier(id) = obj_expr
            && id.name.as_str() == "Number"
        {
            match method_name {
                "isNaN" => return Some(BuiltinCall::NumberIsNaN),
                "isFinite" => return Some(BuiltinCall::NumberIsFinite),
                "isInteger" => return Some(BuiltinCall::NumberIsInteger),
                "isSafeInteger" => return Some(BuiltinCall::NumberIsSafeInteger),
                "parseInt" => return Some(BuiltinCall::NumberParseInt),
                "parseFloat" => return Some(BuiltinCall::NumberParseFloat),
                _ => return None,
            }
        }

        // Check if object is "String" (for String static methods)
        if let Expression::Identifier(id) = obj_expr
            && id.name.as_str() == "String"
        {
            match method_name {
                "fromCharCode" => return Some(BuiltinCall::StringFromCharCode),
                "fromCodePoint" => return Some(BuiltinCall::StringFromCodePoint),
                _ => return None,
            }
        }

        // Check if object is "Array" (for Array static methods)
        if let Expression::Identifier(id) = obj_expr
            && id.name.as_str() == "Array"
        {
            match method_name {
                "from" => return Some(BuiltinCall::ArrayFrom),
                "of" => return Some(BuiltinCall::ArrayOf),
                "isArray" => return Some(BuiltinCall::ArrayIsArray),
                _ => return None,
            }
        }

        // Check if object is a string literal (for String methods)
        let is_string = matches!(obj_expr, Expression::StringLiteral(_));

        // Check if object is an array literal (for Array methods)
        let is_array = matches!(obj_expr, Expression::ArrayExpression(_));

        // Handle array-specific methods (for array literals)
        if is_array {
            match method_name {
                "keys" => return Some(BuiltinCall::ArrayKeys),
                "values" => return Some(BuiltinCall::ArrayValues),
                "entries" => return Some(BuiltinCall::ArrayEntries),
                _ => {}
            }
        }

        // Detect methods based on object type and method name
        match method_name {
            // String-specific methods (always String methods)
            "startsWith" => Some(BuiltinCall::StringStartsWith),
            "endsWith" => Some(BuiltinCall::StringEndsWith),
            "trim" => Some(BuiltinCall::StringTrim),
            "trimStart" => Some(BuiltinCall::StringTrimStart),
            "trimEnd" => Some(BuiltinCall::StringTrimEnd),
            "split" => Some(BuiltinCall::StringSplit),
            "padStart" => Some(BuiltinCall::StringPadStart),
            "padEnd" => Some(BuiltinCall::StringPadEnd),
            "toUpperCase" => Some(BuiltinCall::StringToUpperCase),
            "toLowerCase" => Some(BuiltinCall::StringToLowerCase),
            "toLocaleUpperCase" => Some(BuiltinCall::StringToLocaleUpperCase),
            "toLocaleLowerCase" => Some(BuiltinCall::StringToLocaleLowerCase),
            "charAt" => Some(BuiltinCall::StringCharAt),
            "charCodeAt" => Some(BuiltinCall::StringCharCodeAt),
            "codePointAt" => Some(BuiltinCall::StringCodePointAt),
            "replace" => Some(BuiltinCall::StringReplace),
            "replaceAll" => Some(BuiltinCall::StringReplaceAll),
            "repeat" => Some(BuiltinCall::StringRepeat),
            "substring" => Some(BuiltinCall::StringSubstring),
            "match" => Some(BuiltinCall::StringMatch),
            "search" => Some(BuiltinCall::StringSearch),
            "matchAll" => Some(BuiltinCall::StringMatchAll),
            "localeCompare" => Some(BuiltinCall::StringLocaleCompare),
            "normalize" => Some(BuiltinCall::StringNormalize),

            // Methods that exist on both String and Array
            "indexOf" => {
                if is_string {
                    Some(BuiltinCall::StringIndexOf)
                } else {
                    Some(BuiltinCall::ArrayIndexOf)
                }
            }
            "lastIndexOf" => {
                if is_string {
                    Some(BuiltinCall::StringLastIndexOf)
                } else {
                    Some(BuiltinCall::ArrayLastIndexOf)
                }
            }
            "includes" => {
                if is_string {
                    Some(BuiltinCall::StringIncludes)
                } else {
                    Some(BuiltinCall::ArrayIncludes)
                }
            }
            "concat" => {
                if is_string {
                    Some(BuiltinCall::StringConcat)
                } else {
                    Some(BuiltinCall::ArrayConcat)
                }
            }
            "slice" => {
                if is_string {
                    Some(BuiltinCall::StringSlice)
                } else {
                    Some(BuiltinCall::ArraySlice)
                }
            }

            "pop" => Some(BuiltinCall::ArrayPop),
            "shift" => Some(BuiltinCall::ArrayShift),
            "unshift" => Some(BuiltinCall::ArrayUnshift),
            "reverse" => Some(BuiltinCall::ArrayReverse),
            "sort" => Some(BuiltinCall::ArraySort),
            "join" => Some(BuiltinCall::ArrayJoin),
            "splice" => Some(BuiltinCall::ArraySplice),
            "forEach" => Some(BuiltinCall::ArrayForEach),
            "map" => Some(BuiltinCall::ArrayMap),
            "filter" => Some(BuiltinCall::ArrayFilter),
            "reduce" => Some(BuiltinCall::ArrayReduce),
            "some" => Some(BuiltinCall::ArraySome),
            "every" => Some(BuiltinCall::ArrayEvery),
            "flat" => Some(BuiltinCall::ArrayFlat),
            "flatMap" => Some(BuiltinCall::ArrayFlatMap),
            "find" => Some(BuiltinCall::ArrayFind),
            "findIndex" => Some(BuiltinCall::ArrayFindIndex),
            "findLast" => Some(BuiltinCall::ArrayFindLast),
            "findLastIndex" => Some(BuiltinCall::ArrayFindLastIndex),
            "reduceRight" => Some(BuiltinCall::ArrayReduceRight),
            "fill" => Some(BuiltinCall::ArrayFill),
            "at" => {
                if is_string {
                    Some(BuiltinCall::StringAt)
                } else {
                    Some(BuiltinCall::ArrayAt)
                }
            }
            "copyWithin" => Some(BuiltinCall::ArrayCopyWithin),

            // TypedArray-specific methods (non-overlapping with Array)
            "subarray" => Some(BuiltinCall::TypedArraySubarray),
            // copyWithin routes to ArrayCopyWithin (codegen dispatches to TypedArray)

            // Date instance methods (called on a JsDate struct)
            "getTime" => Some(BuiltinCall::DateGetTime),
            "getFullYear" => Some(BuiltinCall::DateGetFullYear),
            "getMonth" => Some(BuiltinCall::DateGetMonth),
            "getDate" => Some(BuiltinCall::DateGetDate),
            "getDay" => Some(BuiltinCall::DateGetDay),
            "getHours" => Some(BuiltinCall::DateGetHours),
            "getMinutes" => Some(BuiltinCall::DateGetMinutes),
            "getSeconds" => Some(BuiltinCall::DateGetSeconds),
            "getMilliseconds" => Some(BuiltinCall::DateGetMilliseconds),
            "getTimezoneOffset" => Some(BuiltinCall::DateGetTimezoneOffset),
            "toISOString" => Some(BuiltinCall::DateToISOString),
            "toString" => Some(BuiltinCall::DateToString),
            "toDateString" => Some(BuiltinCall::DateToDateString),
            "toTimeString" => Some(BuiltinCall::DateToTimeString),
            "toLocaleString" => Some(BuiltinCall::DateToLocaleString),
            "toFixed" => Some(BuiltinCall::NumberToFixed),
            "toExponential" => Some(BuiltinCall::NumberToExponential),
            "toPrecision" => Some(BuiltinCall::NumberToPrecision),
            "getUTCFullYear" => Some(BuiltinCall::DateGetUTCFullYear),
            "getUTCMonth" => Some(BuiltinCall::DateGetUTCMonth),
            "getUTCDate" => Some(BuiltinCall::DateGetUTCDate),
            "getUTCDay" => Some(BuiltinCall::DateGetUTCDay),
            "getUTCHours" => Some(BuiltinCall::DateGetUTCHours),
            "getUTCMinutes" => Some(BuiltinCall::DateGetUTCMinutes),
            "getUTCSeconds" => Some(BuiltinCall::DateGetUTCSeconds),
            "getUTCMilliseconds" => Some(BuiltinCall::DateGetUTCMilliseconds),

            // Date methods (toJSON/valueOf)
            "toJSON" => Some(BuiltinCall::DateToJSON),
            "valueOf" => Some(BuiltinCall::DateValueOf),

            // Date methods (local setters)
            "setFullYear" => Some(BuiltinCall::DateSetFullYear),
            "setMonth" => Some(BuiltinCall::DateSetMonth),
            "setDate" => Some(BuiltinCall::DateSetDate),
            "setHours" => Some(BuiltinCall::DateSetHours),
            "setMinutes" => Some(BuiltinCall::DateSetMinutes),
            "setSeconds" => Some(BuiltinCall::DateSetSeconds),
            "setMilliseconds" => Some(BuiltinCall::DateSetMilliseconds),

            // Date methods (UTC setters)
            "setUTCFullYear" => Some(BuiltinCall::DateSetUTCFullYear),
            "setUTCMonth" => Some(BuiltinCall::DateSetUTCMonth),
            "setUTCDate" => Some(BuiltinCall::DateSetUTCDate),
            "setUTCHours" => Some(BuiltinCall::DateSetUTCHours),
            "setUTCMinutes" => Some(BuiltinCall::DateSetUTCMinutes),
            "setUTCSeconds" => Some(BuiltinCall::DateSetUTCSeconds),
            "setUTCMilliseconds" => Some(BuiltinCall::DateSetUTCMilliseconds),

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
            "clear" => {
                // Could be Map.clear() or Set.clear()
                // Both have identical signatures, shared variant
                Some(BuiltinCall::MapClear)
            }
            "keys" => Some(BuiltinCall::MapKeys),
            "values" => Some(BuiltinCall::MapValues),
            "entries" => Some(BuiltinCall::MapEntries),

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
        | BuiltinCall::MathPow
        | BuiltinCall::MathSin
        | BuiltinCall::MathCos
        | BuiltinCall::MathTan
        | BuiltinCall::MathAsin
        | BuiltinCall::MathAcos
        | BuiltinCall::MathAtan
        | BuiltinCall::MathAtan2
        | BuiltinCall::MathLog
        | BuiltinCall::MathLog10
        | BuiltinCall::MathLog2
        | BuiltinCall::MathExp
        | BuiltinCall::MathSign
        | BuiltinCall::MathTrunc
        | BuiltinCall::MathCbrt
        // Phase 4 Math methods
        | BuiltinCall::MathExpm1
        | BuiltinCall::MathSinh
        | BuiltinCall::MathCosh
        | BuiltinCall::MathTanh
        | BuiltinCall::MathAsinh
        | BuiltinCall::MathAcosh
        | BuiltinCall::MathAtanh
        | BuiltinCall::MathLog1p => Some(ZigType::F64),

        // Math methods with special return types
        BuiltinCall::MathClz32 | BuiltinCall::MathImul => Some(ZigType::I64),
        BuiltinCall::MathFround => Some(ZigType::F64),

        // Math max/min — depends on args, can't statically determine
        BuiltinCall::MathMax | BuiltinCall::MathMin | BuiltinCall::MathHypot => None,

        // String methods
        BuiltinCall::StringIndexOf | BuiltinCall::StringLastIndexOf | BuiltinCall::StringSearch => {
            Some(ZigType::I64)
        }
        BuiltinCall::StringCodePointAt => Some(ZigType::I64),
        BuiltinCall::StringIncludes
        | BuiltinCall::StringStartsWith
        | BuiltinCall::StringEndsWith => Some(ZigType::Bool),
        BuiltinCall::StringTrim
        | BuiltinCall::StringTrimStart
        | BuiltinCall::StringTrimEnd
        | BuiltinCall::StringSplit
        | BuiltinCall::StringToUpperCase
        | BuiltinCall::StringToLowerCase
        | BuiltinCall::StringCharAt
        | BuiltinCall::StringConcat
        | BuiltinCall::StringSlice
        | BuiltinCall::StringReplace
        | BuiltinCall::StringReplaceAll
        | BuiltinCall::StringRepeat
        | BuiltinCall::StringSubstring
        | BuiltinCall::StringAt
        | BuiltinCall::StringToLocaleUpperCase
        | BuiltinCall::StringToLocaleLowerCase => Some(ZigType::Str),
        // charCodeAt returns u16 — no ZigType variant, defer to inference

        // Map methods
        BuiltinCall::MapGet => Some(ZigType::Anytype), // Conservative
        BuiltinCall::MapHas => Some(ZigType::Bool),
        BuiltinCall::MapKeys => Some(ZigType::ArrayList(Box::new(ZigType::Str))),
        BuiltinCall::MapValues | BuiltinCall::MapEntries => None,
        // Set methods
        BuiltinCall::SetKeys | BuiltinCall::SetValues => {
            Some(ZigType::ArrayList(Box::new(ZigType::JsAny)))
        }
        BuiltinCall::SetEntries => {
            Some(ZigType::ArrayList(Box::new(ZigType::ArrayList(Box::new(ZigType::JsAny)))))
        }

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
        | BuiltinCall::DateGetSeconds
        | BuiltinCall::DateGetMilliseconds
        | BuiltinCall::DateGetTimezoneOffset
        | BuiltinCall::DateGetUTCFullYear
        | BuiltinCall::DateGetUTCMonth
        | BuiltinCall::DateGetUTCDate
        | BuiltinCall::DateGetUTCDay
        | BuiltinCall::DateGetUTCHours
        | BuiltinCall::DateGetUTCMinutes
        | BuiltinCall::DateGetUTCSeconds
        | BuiltinCall::DateGetUTCMilliseconds => Some(ZigType::I64),

        // Date string methods
        BuiltinCall::DateToISOString => Some(ZigType::Str),
        BuiltinCall::DateToString => Some(ZigType::Str),
        BuiltinCall::DateToDateString => Some(ZigType::Str),
        BuiltinCall::DateToTimeString => Some(ZigType::Str),
        BuiltinCall::DateToLocaleString => Some(ZigType::Str),
        // Date toJSON/valueOf
        BuiltinCall::DateToJSON => Some(ZigType::Str),
        BuiltinCall::DateValueOf => Some(ZigType::I64),
        // Date setters (return new milliseconds)
        BuiltinCall::DateSetFullYear
        | BuiltinCall::DateSetMonth
        | BuiltinCall::DateSetDate
        | BuiltinCall::DateSetHours
        | BuiltinCall::DateSetMinutes
        | BuiltinCall::DateSetSeconds
        | BuiltinCall::DateSetMilliseconds
        | BuiltinCall::DateSetUTCFullYear
        | BuiltinCall::DateSetUTCMonth
        | BuiltinCall::DateSetUTCDate
        | BuiltinCall::DateSetUTCHours
        | BuiltinCall::DateSetUTCMinutes
        | BuiltinCall::DateSetUTCSeconds
        | BuiltinCall::DateSetUTCMilliseconds => Some(ZigType::I64),

        // Object methods
        BuiltinCall::ObjectKeys | BuiltinCall::ObjectValues | BuiltinCall::ObjectEntries => {
            Some(ZigType::ArrayList(Box::new(ZigType::Str)))
        }
        BuiltinCall::ObjectHasOwn | BuiltinCall::ObjectIs => Some(ZigType::Bool),
        BuiltinCall::ObjectGetOwnPropertyNames => Some(ZigType::ArrayList(Box::new(ZigType::Str))),
        // Object methods that return complex types or the input object
        BuiltinCall::ObjectSeal | BuiltinCall::ObjectCreate | BuiltinCall::ObjectDefineProperty | BuiltinCall::ObjectGetPrototypeOf | BuiltinCall::ObjectDefineProperties | BuiltinCall::ObjectGetOwnPropertyDescriptor | BuiltinCall::ObjectSetPrototypeOf => None,

        // Array static methods
        BuiltinCall::ArrayFrom | BuiltinCall::ArrayOf => {
            Some(ZigType::ArrayList(Box::new(ZigType::Anytype)))
        }
        BuiltinCall::ArrayIsArray => Some(ZigType::Bool),

        // Array methods — indexOf-type
        BuiltinCall::ArrayIndexOf | BuiltinCall::ArrayLastIndexOf | BuiltinCall::ArrayFindIndex | BuiltinCall::ArrayFindLastIndex => Some(ZigType::I64),

        // Array iterator methods
        BuiltinCall::ArrayKeys | BuiltinCall::ArrayValues | BuiltinCall::ArrayEntries => {
            Some(ZigType::ArrayList(Box::new(ZigType::JsAny)))
        }

        // Global functions
        BuiltinCall::ParseInt => Some(ZigType::I64),
        BuiltinCall::ParseFloat => Some(ZigType::F64),
        BuiltinCall::IsNaN | BuiltinCall::IsFinite => Some(ZigType::Bool),
        BuiltinCall::EncodeURIComponent | BuiltinCall::DecodeURIComponent => Some(ZigType::Str),
        BuiltinCall::EncodeURI | BuiltinCall::DecodeURI => Some(ZigType::Str),

        // Number static methods
        BuiltinCall::NumberIsNaN
        | BuiltinCall::NumberIsFinite
        | BuiltinCall::NumberIsInteger
        | BuiltinCall::NumberIsSafeInteger => Some(ZigType::Bool),
        BuiltinCall::NumberParseInt => Some(ZigType::I64),
        BuiltinCall::NumberParseFloat => Some(ZigType::F64),

        // String static methods
        BuiltinCall::StringFromCharCode | BuiltinCall::StringFromCodePoint => Some(ZigType::Str),

        // Number instance methods
        BuiltinCall::NumberToFixed => Some(ZigType::Str),
        BuiltinCall::NumberToExponential => Some(ZigType::Str),
        BuiltinCall::NumberToPrecision => Some(ZigType::Str),

        // JSON methods
        BuiltinCall::JsonStringify => Some(ZigType::Str), // Returns JSON string
        BuiltinCall::JsonParse => Some(ZigType::JsAny),   // Returns dynamic JSON value

        // Methods that return void or complex types — can't infer
        _ => None,
    }
}
