// native_builtins.rs — moved from native_proto/builtins.rs
// Built-in object methods (Math, Array, String, etc.)
//
// This module only defines the BuiltinCall enum and detection function.
// The emission logic is in zigir::emit::builtins (since it needs to call private methods).

use crate::types::ZigType;

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
    MathHypot,  // Math.hypot(...v) — sqrt(sum of squares)
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
    ArrayPush,        // arr.push(x)
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

    // Array ES2023 immutable methods
    ArrayWith,       // arr.with(index, value) — replace element at index, return new array
    ArrayToReversed, // arr.toReversed() — return reversed copy
    ArrayToSorted,   // arr.toSorted(compareFn) — return sorted copy
    ArrayToSpliced,  // arr.toSpliced(start, deleteCount, ...items) — return spliced copy

    // Array static methods
    ArrayFrom,    // Array.from(arrayLike[, mapFn[, thisArg]])
    ArrayOf,      // Array.of(...items)
    ArrayIsArray, // Array.isArray(obj)

    // TypedArray methods (.get/.set routed through MapGet/MapSet in the Emitter,
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
    StringMatch,       // str.match(regex) — host function
    StringSearch,      // str.search(regex) — host function

    // RegExp instance methods (host function, mini implementation)
    RegExpTest, // /pattern/.test(str) → bool
    RegExpExec, // /pattern/.exec(str) → result (deferred)

    // Map methods (called on local Map variables)
    MapSet,     // map.set(key, value)
    MapGet,     // map.get(key)
    MapHas,     // map.has(key) or set.has(value)
    MapDelete,  // map.delete(key) or set.delete(value)
    MapKeys,    // map.keys() → string[]
    MapValues,  // map.values() → any[]
    MapEntries, // map.entries() → [string, any][]

    // Set methods (called on local Set variables)
    SetAdd, // set.add(value)

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
    DateSetTime,            // date.setTime(ms)
    DateToUTCString,        // date.toUTCString()

    // Object methods (static)
    ObjectKeys,                     // Object.keys(obj)
    ObjectValues,                   // Object.values(obj)
    ObjectEntries,                  // Object.entries(obj)
    ObjectFromEntries,              // Object.fromEntries(iterable)
    ObjectAssign,                   // Object.assign(target, source)
    ObjectFreeze,                   // Object.freeze(obj)
    ObjectSeal,                     // Object.seal(obj) — simplified no-op
    ObjectPreventExtensions,        // Object.preventExtensions(obj)
    ObjectHasOwn,                   // Object.hasOwn(obj, key)
    ObjectIs,                       // Object.is(a, b) — SameValue comparison
    ObjectGetOwnPropertyNames,      // Object.getOwnPropertyNames(obj)
    ObjectCreate,                   // Object.create(proto)
    ObjectDefineProperty,           // Object.defineProperty(obj, key, desc)
    ObjectGetPrototypeOf,           // Object.getPrototypeOf(obj)
    ObjectDefineProperties,         // Object.defineProperties(obj, props)
    ObjectGetOwnPropertyDescriptor, // Object.getOwnPropertyDescriptor(obj, key)
    ObjectSetPrototypeOf,           // Object.setPrototypeOf(obj, proto) — simplified no-op
    ObjectIsSealed,                 // Object.isSealed(obj) — always true in Zig
    ObjectIsFrozen,                 // Object.isFrozen(obj) — always true in Zig
    ObjectIsExtensible,             // Object.isExtensible(obj) — always false in Zig
    ObjectGroupBy,                  // Object.groupBy(items, fn) — ES2024

    // Global functions
    ParseInt,           // parseInt(s)
    ParseFloat,         // parseFloat(s)
    IsNaN,              // isNaN(v)
    IsFinite,           // isFinite(v)
    EncodeURIComponent, // encodeURIComponent(s)
    DecodeURIComponent, // decodeURIComponent(s)
    EncodeURI,          // encodeURI(s)
    DecodeURI,          // decodeURI(s)

    // Global type constructors (used as functions)
    NumberConstructor,    // Number(x) → f64
    StringConstructor,    // String(x) → []const u8
    BooleanConstructor,   // Boolean(x) → bool
    BigIntConstructor,    // BigInt(x) → BigInt
    BigIntToString,       // bigint.toString() → str
    BigIntValueOf,        // bigint.valueOf() → BigInt
    BigIntToLocaleString, // bigint.toLocaleString() → str
    BigIntAsIntN,         // BigInt.asIntN(width, bigint) → BigInt
    BigIntAsUintN,        // BigInt.asUintN(width, bigint) → BigInt
    ObjectConstructor,    // Object(x) → JsAny (wrapping primitive to object)

    // Unsupported global functions (emit @compileError)
    Eval, // eval(s) — security risk, not supported

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

    // String methods (locale-sensitive / ICU-dependent)
    StringMatchAll, // str.matchAll(regex) — returns array of match arrays with capture groups
    StringLocaleCompare, // str.localeCompare(other) — ICU4X-backed when needs_icu, else simplified
    StringNormalize, // str.normalize(form) — ICU4X-backed when needs_icu, else pass-through
    StringToLocaleUpperCase, // str.toLocaleUpperCase() — ICU4X-backed when needs_icu, else simplified
    StringToLocaleLowerCase, // str.toLocaleLowerCase() — ICU4X-backed when needs_icu, else simplified

    // Map/Set clear (shared variant like MapHas/MapDelete)
    MapClear, // map.clear() or set.clear()

    // JSON methods
    JsonStringify, // JSON.stringify(value, replacer?, space?)
    JsonParse,     // JSON.parse(text, reviver?)

    // Symbol methods (static)
    SymbolConstructor, // Symbol(description?)
    SymbolFor,         // Symbol.for(key)
    SymbolKeyFor,      // Symbol.keyFor(sym)
}

/// Check if an identifier name is a JavaScript built-in (global function, constructor, object, or constant).
/// Used to filter out builtins from closure captures — builtins are not user variables and should not
/// become struct fields in generated Zig closure types.
pub fn is_js_builtin_identifier(name: &str) -> bool {
    matches!(
        name,
        // Global functions
        "parseInt" | "parseFloat" | "isNaN" | "isFinite"
        | "encodeURI" | "decodeURI"
        | "encodeURIComponent" | "decodeURIComponent"
        | "eval"
        // Global objects
        | "Math" | "Date" | "Object" | "JSON" | "console"
        | "Number" | "String" | "Boolean" | "BigInt"
        | "Array" | "Symbol" | "RegExp"
        | "Map" | "Set" | "WeakMap" | "WeakSet"
        | "Promise" | "ArrayBuffer" | "DataView"
        // Error constructors
        | "Error" | "TypeError" | "RangeError"
        | "SyntaxError" | "ReferenceError"
        // Global constants
        | "NaN" | "Infinity" | "undefined" | "globalThis"
    )
}

/// Check if a call expression is a built-in object call
/// Returns Some(BuiltinCall) if it is, None otherwise
pub fn detect_builtin_call(ce: &oxc_ast::ast::CallExpression) -> Option<BuiltinCall> {
    use oxc_ast::ast::*;

    // Global function calls (plain identifier callee)
    if let Expression::Identifier(id) = &ce.callee {
        return match id.name.as_str() {
            "parseInt" => Some(BuiltinCall::ParseInt),
            "parseFloat" => Some(BuiltinCall::ParseFloat),
            "isNaN" => Some(BuiltinCall::IsNaN),
            "isFinite" => Some(BuiltinCall::IsFinite),
            "encodeURIComponent" => Some(BuiltinCall::EncodeURIComponent),
            "decodeURIComponent" => Some(BuiltinCall::DecodeURIComponent),
            "encodeURI" => Some(BuiltinCall::EncodeURI),
            "decodeURI" => Some(BuiltinCall::DecodeURI),
            "eval" => Some(BuiltinCall::Eval),
            "Number" => Some(BuiltinCall::NumberConstructor),
            "String" => Some(BuiltinCall::StringConstructor),
            "Boolean" => Some(BuiltinCall::BooleanConstructor),
            "BigInt" => Some(BuiltinCall::BigIntConstructor),
            "Object" => Some(BuiltinCall::ObjectConstructor),
            "Symbol" => Some(BuiltinCall::SymbolConstructor),
            _ => None,
        };
    }

    // Check if callee is a StaticMemberExpression (obj.method())
    if let Expression::StaticMemberExpression(mem) = &ce.callee {
        // Get object expression
        let obj_expr = &mem.object;

        // Get method name
        let method_name = mem.property.name.as_str();

        // Check if object is "BigInt" (for BigInt static methods)
        if let Expression::Identifier(id) = obj_expr
            && id.name.as_str() == "BigInt"
        {
            return match method_name {
                "asIntN" => Some(BuiltinCall::BigIntAsIntN),
                "asUintN" => Some(BuiltinCall::BigIntAsUintN),
                _ => None,
            };
        }

        // Check if object is "Math" (for Math methods)
        if let Expression::Identifier(id) = obj_expr
            && id.name.as_str() == "Math"
        {
            // Math methods
            return match method_name {
                "abs" => Some(BuiltinCall::MathAbs),
                "floor" => Some(BuiltinCall::MathFloor),
                "ceil" => Some(BuiltinCall::MathCeil),
                "round" => Some(BuiltinCall::MathRound),
                "sqrt" => Some(BuiltinCall::MathSqrt),
                "random" => Some(BuiltinCall::MathRandom),
                "pow" => Some(BuiltinCall::MathPow),
                "max" => Some(BuiltinCall::MathMax),
                "min" => Some(BuiltinCall::MathMin),
                "hypot" => Some(BuiltinCall::MathHypot),
                "sin" => Some(BuiltinCall::MathSin),
                "cos" => Some(BuiltinCall::MathCos),
                "tan" => Some(BuiltinCall::MathTan),
                "asin" => Some(BuiltinCall::MathAsin),
                "acos" => Some(BuiltinCall::MathAcos),
                "atan" => Some(BuiltinCall::MathAtan),
                "atan2" => Some(BuiltinCall::MathAtan2),
                "log" => Some(BuiltinCall::MathLog),
                "log10" => Some(BuiltinCall::MathLog10),
                "log2" => Some(BuiltinCall::MathLog2),
                "exp" => Some(BuiltinCall::MathExp),
                "sign" => Some(BuiltinCall::MathSign),
                "trunc" => Some(BuiltinCall::MathTrunc),
                "cbrt" => Some(BuiltinCall::MathCbrt),
                // Phase 4 Math methods
                "expm1" => Some(BuiltinCall::MathExpm1),
                "sinh" => Some(BuiltinCall::MathSinh),
                "cosh" => Some(BuiltinCall::MathCosh),
                "tanh" => Some(BuiltinCall::MathTanh),
                "asinh" => Some(BuiltinCall::MathAsinh),
                "acosh" => Some(BuiltinCall::MathAcosh),
                "atanh" => Some(BuiltinCall::MathAtanh),
                "clz32" => Some(BuiltinCall::MathClz32),
                "fround" => Some(BuiltinCall::MathFround),
                "imul" => Some(BuiltinCall::MathImul),
                "log1p" => Some(BuiltinCall::MathLog1p),
                _ => None,
            };
        }

        // Check if object is "Date" (for Date static methods)
        if let Expression::Identifier(id) = obj_expr
            && id.name.as_str() == "Date"
        {
            return match method_name {
                "now" => Some(BuiltinCall::DateNow),
                "parse" => Some(BuiltinCall::DateParse),
                "UTC" => Some(BuiltinCall::DateUTC),
                _ => None,
            };
        }

        // Check if object is "Object" (for Object static methods)
        if let Expression::Identifier(id) = obj_expr
            && id.name.as_str() == "Object"
        {
            return match method_name {
                "keys" => Some(BuiltinCall::ObjectKeys),
                "values" => Some(BuiltinCall::ObjectValues),
                "entries" => Some(BuiltinCall::ObjectEntries),
                "fromEntries" => Some(BuiltinCall::ObjectFromEntries),
                "assign" => Some(BuiltinCall::ObjectAssign),
                "freeze" => Some(BuiltinCall::ObjectFreeze),
                "seal" => Some(BuiltinCall::ObjectSeal),
                "preventExtensions" => Some(BuiltinCall::ObjectPreventExtensions),
                "hasOwn" => Some(BuiltinCall::ObjectHasOwn),
                "is" => Some(BuiltinCall::ObjectIs),
                "getOwnPropertyNames" => Some(BuiltinCall::ObjectGetOwnPropertyNames),
                "create" => Some(BuiltinCall::ObjectCreate),
                "defineProperty" => Some(BuiltinCall::ObjectDefineProperty),
                "getPrototypeOf" => Some(BuiltinCall::ObjectGetPrototypeOf),
                "defineProperties" => Some(BuiltinCall::ObjectDefineProperties),
                "getOwnPropertyDescriptor" => Some(BuiltinCall::ObjectGetOwnPropertyDescriptor),
                "setPrototypeOf" => Some(BuiltinCall::ObjectSetPrototypeOf),
                "isSealed" => Some(BuiltinCall::ObjectIsSealed),
                "isFrozen" => Some(BuiltinCall::ObjectIsFrozen),
                "isExtensible" => Some(BuiltinCall::ObjectIsExtensible),
                "groupBy" => Some(BuiltinCall::ObjectGroupBy),
                _ => None,
            };
        }

        // Check if object is "JSON" (for JSON methods)
        if let Expression::Identifier(id) = obj_expr
            && id.name.as_str() == "JSON"
        {
            return match method_name {
                "stringify" => Some(BuiltinCall::JsonStringify),
                "parse" => Some(BuiltinCall::JsonParse),
                _ => None,
            };
        }

        // Check if object is "console" (for console methods)
        if let Expression::Identifier(id) = obj_expr
            && id.name.as_str() == "console"
        {
            return match method_name {
                "log" => Some(BuiltinCall::ConsoleLog),
                "error" => Some(BuiltinCall::ConsoleError),
                "warn" => Some(BuiltinCall::ConsoleWarn),
                _ => None,
            };
        }

        // Check if object is a RegExp literal (for /pattern/.test(str))
        if let Expression::RegExpLiteral(_re) = obj_expr {
            return match method_name {
                "test" => Some(BuiltinCall::RegExpTest),
                "exec" => Some(BuiltinCall::RegExpExec),
                _ => None,
            };
        }

        // Check if object is "Number" (for Number static methods)
        if let Expression::Identifier(id) = obj_expr
            && id.name.as_str() == "Number"
        {
            return match method_name {
                "isNaN" => Some(BuiltinCall::NumberIsNaN),
                "isFinite" => Some(BuiltinCall::NumberIsFinite),
                "isInteger" => Some(BuiltinCall::NumberIsInteger),
                "isSafeInteger" => Some(BuiltinCall::NumberIsSafeInteger),
                "parseInt" => Some(BuiltinCall::NumberParseInt),
                "parseFloat" => Some(BuiltinCall::NumberParseFloat),
                _ => None,
            };
        }

        // Check if object is "String" (for String static methods)
        if let Expression::Identifier(id) = obj_expr
            && id.name.as_str() == "String"
        {
            return match method_name {
                "fromCharCode" => Some(BuiltinCall::StringFromCharCode),
                "fromCodePoint" => Some(BuiltinCall::StringFromCodePoint),
                _ => None,
            };
        }

        // Check if object is "Array" (for Array static methods)
        if let Expression::Identifier(id) = obj_expr
            && id.name.as_str() == "Array"
        {
            return match method_name {
                "from" => Some(BuiltinCall::ArrayFrom),
                "of" => Some(BuiltinCall::ArrayOf),
                "isArray" => Some(BuiltinCall::ArrayIsArray),
                _ => None,
            };
        }

        // Check if object is "Symbol" (for Symbol static methods)
        if let Expression::Identifier(id) = obj_expr
            && id.name.as_str() == "Symbol"
        {
            return match method_name {
                "for" => Some(BuiltinCall::SymbolFor),
                "keyFor" => Some(BuiltinCall::SymbolKeyFor),
                _ => None,
            };
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
            "push" => Some(BuiltinCall::ArrayPush),
            "shift" => Some(BuiltinCall::ArrayShift),
            "unshift" => Some(BuiltinCall::ArrayUnshift),
            "reverse" => Some(BuiltinCall::ArrayReverse),
            "sort" => Some(BuiltinCall::ArraySort),
            "join" => Some(BuiltinCall::ArrayJoin),
            "splice" => Some(BuiltinCall::ArraySplice),
            "forEach" => {
                // Could be Array.forEach(), Map.forEach() or Set.forEach()
                // Default to ArrayForEach; the Lowerer resolves the actual
                // CollectionKind from var_types at callback-inline time.
                Some(BuiltinCall::ArrayForEach)
            }
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
            "with" => Some(BuiltinCall::ArrayWith),
            "toReversed" => Some(BuiltinCall::ArrayToReversed),
            "toSorted" => Some(BuiltinCall::ArrayToSorted),
            "toSpliced" => Some(BuiltinCall::ArrayToSpliced),
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
            // copyWithin routes to ArrayCopyWithin (Emitter dispatches to TypedArray)

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
            "setTime" => Some(BuiltinCall::DateSetTime),
            "toUTCString" => Some(BuiltinCall::DateToUTCString),

            // Map methods (called on local Map variables)
            "set" => Some(BuiltinCall::MapSet),
            "get" => Some(BuiltinCall::MapGet),
            "has" => {
                // Could be Map.has() or Set.has()
                // Default to Map.has(), will be resolved in the Emitter
                Some(BuiltinCall::MapHas)
            }
            "delete" => {
                // Could be Map.delete() or Set.delete()
                // Default to Map.delete(), will be resolved in the Emitter
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

        // Math max/min/hypot — all return f64
        BuiltinCall::MathHypot => Some(ZigType::F64),

        // String methods
        BuiltinCall::StringIndexOf | BuiltinCall::StringLastIndexOf | BuiltinCall::StringSearch => {
            Some(ZigType::I64)
        }
        BuiltinCall::StringMatch => None, // returns ?[][]const u8 — complex type, defer to inference
        BuiltinCall::StringMatchAll => Some(ZigType::JsAny), // returns JsAny array of arrays
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

        // RegExp instance methods
        BuiltinCall::RegExpTest => Some(ZigType::Bool),
        BuiltinCall::RegExpExec => Some(ZigType::JsAny), // Returns array-like object or null

        // Map methods
        BuiltinCall::MapGet => Some(ZigType::JsAny), // JsMap.get() returns JsAny (undefined if not found)
        BuiltinCall::MapHas => Some(ZigType::Bool),
        BuiltinCall::MapKeys => Some(ZigType::ArrayList(Box::new(ZigType::Str))),

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
        BuiltinCall::DateToUTCString => Some(ZigType::Str),
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
        | BuiltinCall::DateSetUTCMilliseconds
        | BuiltinCall::DateSetTime => Some(ZigType::I64),

        // Object methods
        BuiltinCall::ObjectKeys | BuiltinCall::ObjectGetOwnPropertyNames => {
            Some(ZigType::ArrayList(Box::new(ZigType::Str)))
        }
        BuiltinCall::ObjectValues | BuiltinCall::ObjectEntries => {
            Some(ZigType::ArrayList(Box::new(ZigType::JsAny)))
        }
        BuiltinCall::ObjectHasOwn | BuiltinCall::ObjectIs => Some(ZigType::Bool),
        // Object.freeze/assign return the first argument — type depends on
        // what was passed, so we cannot determine it statically.
        BuiltinCall::ObjectFreeze | BuiltinCall::ObjectAssign => None,
        // Object methods that return complex types or the input object
        BuiltinCall::ObjectSeal | BuiltinCall::ObjectPreventExtensions | BuiltinCall::ObjectCreate | BuiltinCall::ObjectFromEntries | BuiltinCall::ObjectDefineProperty | BuiltinCall::ObjectGetPrototypeOf | BuiltinCall::ObjectDefineProperties | BuiltinCall::ObjectGetOwnPropertyDescriptor | BuiltinCall::ObjectSetPrototypeOf => None,
        BuiltinCall::ObjectIsSealed
        | BuiltinCall::ObjectIsFrozen
        | BuiltinCall::ObjectIsExtensible => Some(ZigType::Bool),
        BuiltinCall::ObjectGroupBy => Some(ZigType::JsAny),

        // Array static methods
        BuiltinCall::ArrayFrom | BuiltinCall::ArrayOf => {
            Some(ZigType::ArrayList(Box::new(ZigType::Anytype)))
        }
        BuiltinCall::ArrayIsArray => Some(ZigType::Bool),
        BuiltinCall::ArrayIncludes | BuiltinCall::ArraySome | BuiltinCall::ArrayEvery => Some(ZigType::Bool),

        // Array methods — indexOf-type
        BuiltinCall::ArrayIndexOf | BuiltinCall::ArrayLastIndexOf | BuiltinCall::ArrayFindIndex | BuiltinCall::ArrayFindLastIndex => Some(ZigType::I64),

        // Array iterator methods
        BuiltinCall::ArrayKeys | BuiltinCall::ArrayValues | BuiltinCall::ArrayEntries => {
            Some(ZigType::ArrayList(Box::new(ZigType::JsAny)))
        }

        // Global functions
        BuiltinCall::ParseInt => Some(ZigType::F64),
        BuiltinCall::ParseFloat => Some(ZigType::F64),
        BuiltinCall::IsNaN | BuiltinCall::IsFinite => Some(ZigType::Bool),
        BuiltinCall::EncodeURIComponent | BuiltinCall::DecodeURIComponent => Some(ZigType::Str),
        BuiltinCall::EncodeURI | BuiltinCall::DecodeURI => Some(ZigType::Str),

        // Number static methods
        BuiltinCall::NumberIsNaN
        | BuiltinCall::NumberIsFinite
        | BuiltinCall::NumberIsInteger
        | BuiltinCall::NumberIsSafeInteger => Some(ZigType::Bool),
        BuiltinCall::NumberParseInt => Some(ZigType::F64),
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

        // Symbol methods
        BuiltinCall::SymbolConstructor | BuiltinCall::SymbolFor => Some(ZigType::JsSymbol),
        BuiltinCall::SymbolKeyFor => Some(ZigType::Str), // Returns ?[]const u8 (description or null)

        // Global type constructors (used as functions)
        BuiltinCall::NumberConstructor => Some(ZigType::F64),
        BuiltinCall::StringConstructor => Some(ZigType::Str),
        BuiltinCall::BooleanConstructor => Some(ZigType::Bool),
        BuiltinCall::BigIntConstructor => Some(ZigType::BigInt),
        BuiltinCall::BigIntToString => Some(ZigType::Str),
        BuiltinCall::BigIntValueOf => Some(ZigType::BigInt),
        BuiltinCall::BigIntToLocaleString => Some(ZigType::Str),
        BuiltinCall::BigIntAsIntN | BuiltinCall::BigIntAsUintN => Some(ZigType::BigInt),
        BuiltinCall::ObjectConstructor => Some(ZigType::JsAny),

        // Math methods — always return Number
        BuiltinCall::MathMax | BuiltinCall::MathMin => Some(ZigType::F64),

        // Array mutation methods that return a known scalar type
        BuiltinCall::ArrayPush | BuiltinCall::ArrayUnshift => Some(ZigType::I64), // new length
        BuiltinCall::ArrayJoin => Some(ZigType::Str),

        // String methods with deterministic return types
        BuiltinCall::StringCharCodeAt => Some(ZigType::F64),     // Number (0-65535 or NaN)
        BuiltinCall::StringLocaleCompare => Some(ZigType::I64),  // Number (negative/0/positive)
        BuiltinCall::StringNormalize => Some(ZigType::Str),      // Returns string (stub: pass-through)

        // Map/Set methods
        BuiltinCall::MapDelete => Some(ZigType::Bool),           // Returns whether element was removed
        BuiltinCall::MapValues => Some(ZigType::ArrayList(Box::new(ZigType::JsAny))),
        BuiltinCall::MapEntries => Some(ZigType::ArrayList(Box::new(ZigType::ArrayList(Box::new(ZigType::JsAny))))),

        // String padding methods
        BuiltinCall::StringPadStart | BuiltinCall::StringPadEnd => Some(ZigType::Str),

        // Map/Set mutation methods that return the receiver (for chaining)
        BuiltinCall::MapSet | BuiltinCall::SetAdd => Some(ZigType::JsAny), // returns receiver

        // Array methods returning the mutated array (JsAny — type depends on input)
        BuiltinCall::ArrayReverse
        | BuiltinCall::ArraySort
        | BuiltinCall::ArrayCopyWithin
        | BuiltinCall::ArrayFill => Some(ZigType::JsAny),

        // Array methods returning a new array
        BuiltinCall::ArraySlice | BuiltinCall::ArrayConcat => {
            Some(ZigType::ArrayList(Box::new(ZigType::Anytype)))
        }
        BuiltinCall::ArraySplice => Some(ZigType::ArrayList(Box::new(ZigType::Anytype))), // deleted elements
        BuiltinCall::ArrayFilter => Some(ZigType::ArrayList(Box::new(ZigType::Anytype))),
        BuiltinCall::ArrayMap | BuiltinCall::ArrayFlatMap | BuiltinCall::ArrayFlat => {
            Some(ZigType::ArrayList(Box::new(ZigType::Anytype)))
        }

        // Array methods returning an element or unknown type
        BuiltinCall::ArrayPop | BuiltinCall::ArrayShift => Some(ZigType::JsAny),
        BuiltinCall::ArrayAt | BuiltinCall::ArrayFind | BuiltinCall::ArrayFindLast => Some(ZigType::JsAny),
        BuiltinCall::ArrayReduce | BuiltinCall::ArrayReduceRight => Some(ZigType::JsAny), // depends on callback

        // ES2023 immutable array methods — return new array
        BuiltinCall::ArrayWith | BuiltinCall::ArrayToReversed | BuiltinCall::ArrayToSorted | BuiltinCall::ArrayToSpliced => {
            Some(ZigType::ArrayList(Box::new(ZigType::Anytype)))
        }

        // TypedArray view method
        BuiltinCall::TypedArraySubarray => Some(ZigType::JsAny), // returns a view — type depends on input

        // Methods that return void or complex types — can't infer
        _ => None,
    }
}
