// zigir/kinds.rs
// Classification enums for member access, calls, and computed keys.

/// How a field access (`obj.field`) should be emitted in Zig.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FieldKind {
    /// Direct struct field: `obj.field`
    StructField,
    /// Namespace/module access: `std.math.pi` (chained field access on import)
    Namespace,
    /// ArrayList length: `arr.items.len`
    ArrayListLen,
    /// String length: `str.len`
    StringLen,
    /// Map/Set size: `map.size()` or `set.size()`  (method call, not field)
    MapSetSize,
    /// Math constant: `std.math.pi`, etc.
    MathConstant(String),
    /// Number constant: `std.math.floatMax(f64)`, etc.
    NumberConstant(String),
    /// Well-known Symbol: `js_symbol.symbolIterator()`, etc.
    SymbolWellKnown(String),
    /// TypedArray property: `.buffer`, `.byteLength`, `.byteOffset`
    TypedArrayProp(String),
    /// Private class field: `self.field` (from #field syntax)
    Private,
    /// Pointer dereference field: `obj.field.*` (captured mutable closure var)
    PointerDeref,
}

/// How an index access (`obj[idx]`) should be emitted in Zig.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndexKind {
    /// ArrayList item: `arr.items[n]`
    ArrayListItem,
    /// Slice index: `arr[n]`
    SliceIndex,
}

/// How a computed member access (`obj[key]`) should be emitted.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ComputedKeyKind {
    /// Struct field via @field: `@field(obj, key)`
    StructField,
    /// Map get: `obj.get(key)`
    MapGet,
    /// JsAny dynamic: `obj.getByKey(key, alloc)`
    JsAnyGetByKey,
    /// ArrayList item: `arr.items[key]`
    ArrayListItem,
    /// Unsupported dynamic access → generates @compileError
    CompileError(String),
}

/// What kind of function call this is.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CallKind {
    /// Direct function call: `fn(args)`
    Direct,
    /// Method call on a known object type: `obj.method(args)`
    Method { object_type: MethodObjectKind },
    /// Closure call: `closure_instance(.call)(args)`
    Closure,
}

/// Known object types for method call dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MethodObjectKind {
    ArrayList,
    String,
    Map,
    Set,
    Date,
    /// User-defined class method: carries the class name.
    Class(String),
    JsAny,
    Unknown,
}

/// What kind of constructor `new X(...)` targets.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NewConstructor {
    Map,
    Set,
    Date(DateConstructorKind),
    RegExp,
    TypedArray(TypedArrayKind),
    /// User-defined class constructor.
    Class(String),
    /// Error constructor: `new Error(msg)`.
    Error(String),
    /// Unsupported constructor → generates @compileError.
    Unsupported(String),
}

/// How `new Date(...)` was called.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DateConstructorKind {
    /// `new Date()` → current time
    Now,
    /// `new Date(millis)` → from milliseconds
    FromMillis,
    /// `new Date(string)` → from string
    FromString,
    /// `new Date(y,m,d,h,min,s,ms)` → from components
    FromComponents,
}

/// TypedArray element types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypedArrayKind {
    Int8Array,
    Uint8Array,
    Uint8ClampedArray,
    Int16Array,
    Uint16Array,
    Int32Array,
    Uint32Array,
    Float32Array,
    Float64Array,
    BigInt64Array,
    BigUint64Array,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_kind_variants() {
        let kinds = [
            FieldKind::StructField,
            FieldKind::ArrayListLen,
            FieldKind::StringLen,
            FieldKind::MapSetSize,
            FieldKind::MathConstant("pi".to_string()),
            FieldKind::NumberConstant("floatMax".to_string()),
            FieldKind::SymbolWellKnown("iterator".to_string()),
            FieldKind::TypedArrayProp("buffer".to_string()),
        ];
        assert_eq!(kinds.len(), 8);
    }

    #[test]
    fn test_call_kind() {
        let direct = CallKind::Direct;
        let method = CallKind::Method {
            object_type: MethodObjectKind::ArrayList,
        };
        let closure = CallKind::Closure;
        assert_ne!(direct, method);
        assert_ne!(method, closure);
    }

    #[test]
    fn test_new_constructor() {
        let date_now = NewConstructor::Date(DateConstructorKind::Now);
        let class = NewConstructor::Class("Foo".to_string());
        assert!(matches!(
            date_now,
            NewConstructor::Date(DateConstructorKind::Now)
        ));
        assert!(matches!(class, NewConstructor::Class(_)));
    }

    #[test]
    fn test_typed_array_kinds() {
        let all = [
            TypedArrayKind::Int8Array,
            TypedArrayKind::Uint8Array,
            TypedArrayKind::Uint8ClampedArray,
            TypedArrayKind::Int16Array,
            TypedArrayKind::Uint16Array,
            TypedArrayKind::Int32Array,
            TypedArrayKind::Uint32Array,
            TypedArrayKind::Float32Array,
            TypedArrayKind::Float64Array,
            TypedArrayKind::BigInt64Array,
            TypedArrayKind::BigUint64Array,
        ];
        assert_eq!(all.len(), 11);
    }
}
