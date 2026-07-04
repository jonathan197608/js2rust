---
AIGC:
  ContentProducer: '001191110102MAD55U9H0F10002'
  ContentPropagator: '001191110102MAD55U9H0F10002'
  Label: '1'
  ProduceID: 'ff9fd553-7012-4418-9315-d073ff6222c4'
  PropagateID: 'ff9fd553-7012-4418-9315-d073ff6222c4'
  ReservedCode1: '7555e344-cd87-4f22-972a-c4c2a3b822d3'
  ReservedCode2: '7555e344-cd87-4f22-972a-c4c2a3b822d3'
---

# Lowerer+Emitter Failure Categorization (145 tests)

Generated: 2026-07-04
Method: Switched `native_proto.rs` to return Emitter output when ZIGIR_DUAL_TRACK=1,
        ran full test suite, captured all failures, categorized by root cause.

---

## 1. Destructuring — 8 tests

Emitter emits `@compileError("unsupported binding pattern in variable declaration")`
instead of expanding array/object destructuring into individual `const` assignments.

| Test | Pattern |
|------|---------|
| tests::destructure_class_arrays::test_p2_destructure_array_basic | `const [a, b] = arr` → should emit `const _js_dest_0 = arr; const a = _js_dest_0[0];` |
| tests::destructure_class_arrays::test_p2_destructure_array_hole | `const [a, , b] = arr` → hole skipping |
| tests::destructure_class_arrays::test_p2_destructure_array_with_defaults | `const [a=1, b=2] = arr` → orelse defaults |
| tests::destructure_class_arrays::test_p2_destructure_object_basic | `const {a, b} = obj` → `obj.get("a")` |
| tests::destructure_class_arrays::test_p2_destructure_object_mixed | Mixed object destructuring with defaults |
| tests::destructure_class_arrays::test_p2_destructure_object_rename | `const {a: x} = obj` → rename |
| tests::destructure_class_arrays::test_p2_destructure_object_with_defaults | `const {a=1, b=2} = obj` → `if (get) |v| v.asI64() else default` |
| tests::destructure_class_arrays::test_p2_destructure_function_call_init | Destructure with function call init |

**Error pattern**: `@compileError("unsupported binding pattern in variable declaration (at ...)")`

---

## 2. Builtin Method Inlining — 29 tests

The old Codegen expands many JS builtin method calls into inline Zig code
(loops, block expressions, allocators, etc.). The Emitter emits a high-level
call like `js_array.indexOf(target)` or `js_string.repeat(n)` instead of the
expanded form. This is the largest category.

### 2a. Array method inlining — 16 tests

| Test | Method | Expected inline pattern |
|------|--------|----------------------|
| tests::builtins_basic::test_native_proto_array_includes | includes | for loop with == check |
| tests::builtins_basic::test_native_proto_array_indexof | indexOf | for loop with break |
| tests::builtins_basic::test_native_proto_array_join | join | std.io.Writer.Allocating init |
| tests::builtins_basic::test_native_proto_array_pop | pop | `arr.pop()` (method on ArrayList, not js_array) |
| tests::builtins_basic::test_native_proto_array_slice | slice | blk with ArrayList append |
| tests::builtins_basic::test_native_proto_array_splice | splice | orderedRemove loop |
| tests::builtins_basic::test_native_proto_array_splice_insert | splice insert | orderedRemove + insert |
| tests::destructure_class_arrays::test_native_proto_array_at | at | `__at_idx` variable with negative index |
| tests::destructure_class_arrays::test_native_proto_array_concat | concat | `__concat` ArrayList var |
| tests::destructure_class_arrays::test_native_proto_array_copywithin | copyWithin | `__cpw_target` inline |
| tests::destructure_class_arrays::test_native_proto_array_fill | fill | for loop with elem.* assignment |
| tests::destructure_class_arrays::test_native_proto_array_fill_range | fill(start,end) | for loop with @intCast |
| tests::destructure_class_arrays::test_native_proto_array_lastindexof | lastIndexOf | backward while loop |
| tests::destructure_class_arrays::test_native_proto_string_at | String.at | `js_string.at(alloc, s, idx)` |
| tests::destructure_class_arrays::test_native_proto_string_match_stub | String.match | `js_string.matchString(alloc, str, pattern)` |
| tests::destructure_class_arrays::test_native_proto_string_search_stub | String.search | `host.regex_search(pattern, str)` |

### 2b. String method inlining — 8 tests

| Test | Method | Expected pattern |
|------|--------|----------------|
| tests::advanced_builtins::test_p6_string_repeat | repeat | `js_string.repeat(js_allocator.allocator(), str, n)` |
| tests::advanced_builtins::test_p6_string_split | split | `js_string.split(js_allocator.allocator(), str, sep)` |
| tests::advanced_builtins::test_p6_string_char_at | charAt | `js_string.charAt(js_allocator.allocator(), str, idx)` |
| tests::advanced_builtins::test_p6_string_normalize | normalize | `js_string.normalize(js_allocator.allocator(), str, form)` |
| tests::advanced_builtins::test_p6_string_to_lower_case | toLowerCase | `js_string.toLower(js_allocator.allocator(), str)` |
| tests::advanced_builtins::test_p6_string_to_upper_case | toUpperCase | `js_string.toUpper(js_allocator.allocator(), str)` |
| tests::advanced_builtins::test_p6_string_to_locale_lower_case | toLocaleLowerCase | `js_string.toLocaleLower(js_allocator.allocator(), str)` |
| tests::advanced_builtins::test_p6_string_to_locale_upper_case | toLocaleUpperCase | `js_string.toLocaleUpper(js_allocator.allocator(), str)` |

### 2c. URI/encode/decode — 4 tests

| Test | Method | Expected pattern |
|------|--------|----------------|
| tests::advanced_builtins::test_p7_encode_uri | encodeURI | `js_uri.encodeURI(js_allocator.allocator(), url) catch @panic(...)` |
| tests::advanced_builtins::test_p7_decode_uri | decodeURI | `js_uri.decodeURI(js_allocator.allocator(), url) catch ""` |
| tests::advanced_builtins::test_p7_encode_uri_component | encodeURIComponent | allocator + catch |
| tests::advanced_builtins::test_p7_decode_uri_component | decodeURIComponent | allocator + catch |

### 2d. Number/Math — 5 tests

| Test | Method | Expected pattern |
|------|--------|----------------|
| tests::advanced_builtins::test_native_proto_number_constants | Number.MAX_VALUE etc. | `std.math.floatMax(f64)`, `floatMin(f64)`, `nan(f64)`, `inf(f64)` |
| tests::advanced_builtins::test_native_proto_number_tofixed | toFixed | `js_number.toFixed(js_allocator.allocator(), pi, 2)` |
| tests::builtins_basic::test_native_proto_math_methods | Math.hypot | Inline `@sqrt(...)` expression |
| tests::builtins_basic::test_native_proto_math_new_methods | Math.random, pow | `std.crypto.random.int(u32)`, `std.math.pow(f64, x, y)` |
| tests::builtins_basic::test_native_proto_math_phase4 | Math.expm1, sinh, cosh, tanh, asinh | `std.math.expm1(x)`, etc. |

### 2e. JSON/stringify — 2 tests

| Test | Method | Expected pattern |
|------|--------|----------------|
| tests::objects_and_types::test_native_proto_typedef_tojson | JSON.stringify | `try js_json.stringify(js_allocator.allocator(), user, null, null)` |
| tests::objects_and_types::test_native_proto_json_parse_nested | JSON.parse | allocator arg + catch |

**Error pattern**: Emitter emits `js_X.method(args)` instead of the fully-expanded
inline code with allocators, block expressions, loops, error handling, etc.

---

## 3. Callback/Arrow Inlining — 12 tests

The old Codegen inlines array callback methods (forEach, every, some, filter, find,
findIndex, findLast, findLastIndex, map) by unwrapping the arrow/block callback into
a for/while loop. The Emitter emits a struct definition in-place as argument, which
produces invalid Zig syntax (struct inside function call).

| Test | Method |
|------|--------|
| tests::destructure_class_arrays::test_native_proto_array_every | every |
| tests::destructure_class_arrays::test_native_proto_array_every_block_body | every (block body) |
| tests::destructure_class_arrays::test_native_proto_array_some | some |
| tests::destructure_class_arrays::test_native_proto_array_some_block_body | some (block body) |
| tests::destructure_class_arrays::test_native_proto_array_filter | filter |
| tests::destructure_class_arrays::test_native_proto_array_find | find |
| tests::destructure_class_arrays::test_native_proto_array_find_block_body | find (block body) |
| tests::destructure_class_arrays::test_native_proto_array_find_index | findIndex |
| tests::destructure_class_arrays::test_native_proto_array_find_index_block_body | findIndex (block body) |
| tests::destructure_class_arrays::test_native_proto_array_find_last | findLast |
| tests::destructure_class_arrays::test_native_proto_array_find_last_index | findLastIndex |
| tests::advanced_builtins::test_p7_set_foreach | Set.forEach |

**Error pattern**: `js_array.every(const _arrow_fn = struct { ... })` — struct emitted inline as argument

**This is a critical blocker**: generates syntactically invalid Zig that won't compile.

---

## 4. RegExp — 8 tests

The Lowerer doesn't properly route RegExp-related calls. It either:
- Emits `js_string.match(@compileError("RegExp literal not supported"))` instead of the
  Codegen's `host.regex_test()` / `js_string.matchString()` paths
- Or emits `js_regexp.test(s)` instead of `host.regex_test(pattern, s)` / `r.isMatch(s)`

| Test | Issue |
|------|-------|
| tests::advanced_builtins::test_p8_regex_test | `js_regexp.test(s)` → should be `host.regex_test("\\d", s)` |
| tests::advanced_builtins::test_p8_new_regexp | `js_regexp.JsRegExp.init("\\d+")` → needs `try` + allocator + `r.isMatch(s)` |
| tests::advanced_builtins::test_p8_regexp_var_test | `r.test(s)` → should be `r.isMatch(s)` |
| tests::advanced_builtins::test_p8_regexp_exec_literal | `js_regexp.exec(s)` → `js_regexp.execLiteral(alloc, s, "world")` |
| tests::advanced_builtins::test_p8_regexp_var_exec | `r.exec(s)` → `r.exec(alloc, s)` |
| tests::advanced_builtins::test_p8_string_match_compile_error | `js_string.match(@compileError(...))` → `js_string.matchString(alloc, s, "hello")` |
| tests::advanced_builtins::test_p8_string_match_regexp_var | regexp var match routing |
| tests::advanced_builtins::test_p8_string_search_regexp_var | `js_string.search(r)` → `host.regex_search(r.pattern, s)` |

**Sub-issue**: 3 more tests with RegExp that involve `matchAll`:
| tests::advanced_builtins::test_p3_string_match_all_ast_check | matchAll with literal |
| tests::advanced_builtins::test_p3_string_match_all_regexp_var_ast_check | matchAll with var |
| tests::not_implemented_and_fixes::test_method_chaining_encodeuri_replace | replace with RegExp literal |
| tests::destructure_class_arrays::test_native_proto_string_search_stub | String.search with literal |
| tests::advanced_builtins::test_p8_string_match_global_ast_check | match with global |
| tests::advanced_builtins::test_p8_string_match_global_empty_match_ast_check | match with global empty |

These 6 overlap with category 2 (Builtin Method Inlining) but the root cause is RegExp routing.

**Error pattern**: Incorrect method routing for RegExp operations; missing allocator/try for JsRegExp.init

---

## 5. Symbol — 6 tests

The Emitter emits the JS-level method name directly (e.g., `js_symbol.iterator()`)
instead of the Zig runtime name (`js_symbol.symbolIterator()`). Also wrong constructor
routing (`js_symbol.constructor()` vs `js_symbol.JsSymbol.initAnonymous()`).

| Test | Emitter emits | Should emit |
|------|---------------|-------------|
| tests::advanced_builtins::test_native_proto_symbol_basic | `js_symbol.constructor()` | `js_symbol.JsSymbol.initAnonymous()` |
| tests::advanced_builtins::test_native_proto_symbol_to_string | wrong body | `sym.toString(alloc)` |
| tests::advanced_builtins::test_native_proto_symbol_for_keyfor | `js_symbol.for(key)` | `js_symbol.symbolFor(key)` — also `for` is a Zig keyword! |
| tests::advanced_builtins::test_native_proto_symbol_well_known_iterator | `js_symbol.iterator()` | `js_symbol.symbolIterator()` |
| tests::advanced_builtins::test_native_proto_symbol_well_known_async_iterator | `js_symbol.asyncIterator()` | `js_symbol.symbolAsyncIterator()` |
| tests::advanced_builtins::test_native_proto_symbol_well_known_multiple | multiple wrong | `symbolIterator`, `symbolMatch`, `symbolToStringTag` |
| tests::advanced_builtins::test_native_proto_symbol_well_known_to_string_tag | `toStringTag()` | `symbolToStringTag()` |

**Error pattern**: JS method names not remapped to Zig runtime names; `for` is a Zig keyword

---

## 6. Optional Chaining — 4 tests

The Lowerer doesn't support optional chaining (`?.`). It emits
`@compileError("unsupported expression type: Unknown")`.

| Test | Expected |
|------|----------|
| tests::try_catch_and_closures::test_native_proto_optional_chain_call | `(if (obj) |_oc0| _oc0.greet("World") else null)` |
| tests::try_catch_and_closures::test_native_proto_optional_chain_known_struct | `obj.name` with optional unwrap |
| tests::try_catch_and_closures::test_native_proto_optional_chain_nested | `(if (obj.a) |_oc0| _oc0.b else null)` |
| tests::try_catch_and_closures::test_native_proto_optional_chain_unknown | `(if (obj) |_oc0| _oc0.name else null)` |

**Error pattern**: `@compileError("unsupported expression type: Unknown (at ...)")`

---

## 7. Exponentiation Operator (`**`) — 4 tests

The Emitter emits `a @pow b` (Zig operator) instead of the Codegen's
`std.math.pow(f64, @as(f64, @floatFromInt(a)), ...)` with proper f64 casting.

| Test | Emitter emits | Should emit |
|------|---------------|-------------|
| tests::try_catch_and_closures::test_native_proto_exponential_operator | `base @pow exp` | `std.math.pow(f64, f64(base), f64(exp))` |
| tests::try_catch_and_closures::test_native_proto_exponential_float | `2 @pow 3` | blk with float cast |
| tests::try_catch_and_closures::test_native_proto_exponential_mixed | `base @pow exp` | blk with float cast |
| tests::try_catch_and_closures::test_native_proto_exponential_edge | `x @pow 0` | blk with edge case handling |

**Error pattern**: Uses `@pow` operator directly without f64 casting/wrapping

---

## 8. Spread/Rest — 7 tests

The Emitter emits `.{ ...a, ...b }` or `.{ .c = 1, ...a }` which is not valid Zig.
The Codegen expands these into `js_runtime.spreadMerge(a, b)` calls.

| Test | Emitter emits | Should emit |
|------|---------------|-------------|
| tests::phase1::test_p1_spread_with_inline | `.{ .extra = 1, ...a }` | `js_runtime.spreadMerge(a, .{ .extra = 1 })` |
| tests::phase1::test_p1_spread_multi | `.{ , ...a, ...b }` | `js_runtime.spreadMerge(a, b)` |
| tests::phase1::test_p1_spread_multi_with_inline | `.{ .c=1, .d="hello", ...a, ...b }` | nested spreadMerge |
| tests::phase1::test_p1_spread_empty | `.{ }` | `std.StringHashMap(JsAny).init(alloc)` |
| tests::phase1::test_p1_call_spread | `foo(arr)` | `foo(arr.items)` with rest param `[]const JsAny` |
| tests::phase1::test_p1_rest_param_and_call_spread | `args: anytype` | `args: []const JsAny` |
| tests::phase1::test_p1_in_operator | `"name" == obj` | `obj.contains("name")` (also in-operator issue) |

**Error pattern**: Spread syntax `...x` emitted directly; rest params not typed as `[]const JsAny`

---

## 9. `instanceof` / `in` / `eval` / `static` — Missing @compileError — 5 tests

The Lowerer doesn't emit `@compileError` for unsupported JS features. Instead it
emits code that silently compiles (or tries to), which produces wrong Zig code.

| Test | Emitter emits | Should emit |
|------|---------------|-------------|
| tests::not_implemented_and_fixes::test_not_implemented_instanceof | `arr == Array` | `@compileError("instanceof operator is not supported")` |
| tests::not_implemented_and_fixes::test_not_implemented_instanceof_with_annotation | `arr == Array` | `@compileError(...)` |
| tests::phase1::test_p1_instanceof_operator | `obj == Array` | `@compileError(...)` |
| tests::not_implemented_and_fixes::test_not_implemented_eval_with_annotation | `js_uri.eval("1+2")` | `@compileError("eval() is not supported")` |
| tests::not_implemented_and_fixes::test_not_implemented_static_block | normal init() | `@compileError("static {} blocks are not supported")` |

**Error pattern**: Emits executable code instead of @compileError for unsupported features

---

## 10. Nested Function Hoisting / Closures — 5 tests

The old Codegen hoists nested function declarations as `const inner = struct { pub fn call(...) }`
and rewrites calls to `inner.call(...)`. The Emitter emits a separate `pub fn inner()`
at the function level, which doesn't capture variables and uses wrong calling convention.

| Test | Issue |
|------|-------|
| tests::basic::test_p2_nested_function_no_capture | Emits separate fn instead of struct |
| tests::basic::test_p2_nested_function_with_capture | Emits separate fn, no capture struct |
| tests::destructure_class_arrays::test_p2_nested_function_basic | Same |
| tests::destructure_class_arrays::test_p2_nested_function_anytype_return | Same |
| tests::destructure_class_arrays::test_p2_nested_function_capture_error | No capture struct with self field |

**Error pattern**: `pub fn inner(y: anytype) anytype { return x + y; }` instead of
`const inner = struct { pub fn call(y: anytype) @TypeOf(y + 1) { ... } };`

---

## 11. Object Builtin Routing — 6 tests

Similar to category 2 (Builtin Method Inlining) but specific to Object.* static methods
where the Emitter emits a direct call without proper argument transformation.

| Test | Emitter emits | Should emit |
|------|---------------|-------------|
| tests::destructure_class_arrays::test_native_proto_object_has_own_struct | `js_object.hasOwn(obj, "name")` | `@hasField(@TypeOf(obj), "name")` |
| tests::destructure_class_arrays::test_native_proto_object_has_own_missing | `js_object.hasOwn(obj, "email")` | `@hasField(@TypeOf(obj), "email")` |
| tests::destructure_class_arrays::test_native_proto_object_is | `js_object.is(a, b)` | `(std.math.isNan(a) and std.math.isNan(b)) or (a == b)` |
| tests::destructure_class_arrays::test_native_proto_object_getownpropertynames_stub | `js_object.getOwnPropertyNames(obj)` | `@compileError("not yet implemented")` |
| tests::phase1::test_p1_object_assign | `js_object.assign(target, source)` | `js_object.assign(&target, &source)` |
| tests::phase1::test_p1_object_freeze | `js_object.freeze(obj)` | `obj` (no-op) |
| tests::phase1::test_p1_object_from_entries | `js_object.fromEntries(entries)` | `js_object.fromEntries(alloc, entries)` |

**Error pattern**: Direct `js_object.X()` call instead of inline expansion / @hasField / allocator arg / no-op

---

## 12. Date Constructor Routing — 7 tests

The Emitter always emits `js_date.JsDate.init()` regardless of `new Date()` arguments.
The Codegen routes to different constructors based on argument count/type.

| Test | Emitter emits | Should emit |
|------|---------------|-------------|
| tests::phase1::test_p1_date_new_millis | `JsDate.init()` | `JsDate.fromMillis(0)` |
| tests::phase1::test_p1_date_new_string | `JsDate.init()` | `JsDate.fromMillis(js_date.parse("2024-01-15"))` |
| tests::phase1::test_p1_date_new_multi_2args | `JsDate.init()` | `JsDate.fromComponents(2024, 5, 1, 0, 0, 0, 0)` |
| tests::phase1::test_p1_date_new_multi_3args | `JsDate.init()` | `JsDate.fromComponents(2024, 5, 15, ...)` |
| tests::phase1::test_p1_date_new_multi_5args | `JsDate.init()` | `JsDate.fromComponents(2024, 5, 15, 12, 30, ...)` |
| tests::phase1::test_p1_date_new_multi_7args | `JsDate.init()` | `JsDate.fromComponents(2024, 5, 15, 12, 30, 45, 500)` |
| tests::phase1::test_p1_date_new_multi_variable_args | `JsDate.init()` | `JsDate.fromComponents(y, m, d, 0, 0, 0, 0)` |

**Error pattern**: All `new Date()` forms emit `JsDate.init()` regardless of arguments

---

## 13. TypedArray — 9 tests

Emitter emits `js_typedarray.JsInt32Array.init(...)` with wrong constructor call
and wrong property/method routing. Codegen uses `js_runtime.js_typedarray.fromI64AsI32(...)`
and routes property accesses to runtime helpers.

| Test | Wrong emit | Correct pattern |
|------|------------|-----------------|
| tests::builtins_basic::test_native_proto_typedarray_basic | `JsInt32Array.init(blk...)` | `js_runtime.js_typedarray.fromI64AsI32(&[_]i64{1,2,3})` |
| tests::builtins_basic::test_native_proto_typedarray_uint8 | `JsUint8Array.init(blk...)` | `js_runtime.js_typedarray.fromI64AsU8(&[_]i64{1,2,3})` |
| tests::builtins_basic::test_native_proto_float64array | `JsFloat64Array.init(blk...)` | `js_runtime.js_typedarray.fromF64(&[_]f64{1.5,2.5,3.5})` |
| tests::builtins_basic::test_native_proto_typedarray_buffer | `arr.buffer` | `js_runtime.js_typedarray.bufferI32(arr)` |
| tests::builtins_basic::test_native_proto_typedarray_bytelength | `arr.byteLength` | `js_runtime.js_typedarray.byteLengthI32(arr)` |
| tests::builtins_basic::test_native_proto_typedarray_set | `js_collections.set(idx, val)` | `js_runtime.js_typedarray.setI32(arr, idx, val)` |
| tests::builtins_basic::test_native_proto_typedarray_slice | `js_array.slice(1, 4)` | `js_runtime.js_typedarray.sliceI32(arr, 1, 4)` |
| tests::builtins_basic::test_native_proto_typedarray_subarray | `js_typedarray.subarray(1, 3)` | `js_runtime.js_typedarray.subarrayI32(arr, 1, 3)` |
| tests::builtins_basic::test_native_proto_typedarray_copywithin | `js_array.copyWithin()` | `js_runtime.js_typedarray.copyWithinI32()` |
| tests::builtins_basic::test_native_proto_typedarray_fill | `js_array.fill()` | `js_runtime.js_typedarray.fillI32()` |

**Error pattern**: Wrong constructor, wrong property accessor routing, `js_collections` instead of `js_typedarray`

---

## 14. Map/Set — 3 tests (overlap with category 3)

| Test | Issue |
|------|-------|
| tests::advanced_builtins::test_p7_set_foreach | forEach callback not inlined |
| tests::advanced_builtins::test_native_proto_map_get_eq_cmp | `js_collections.set()` → `m.set(JsAny.from(x), JsAny.from(y)) catch @panic("OOM")`; `==` → `.eq()` |
| tests::advanced_builtins::test_native_proto_object_is_sealed_frozen_extensible | Emits runtime calls instead of compile-time `true`/`false` |

**Error pattern**: Missing JsAny.from() wrapping, catch @panic, .eq() for comparisons

---

## 15. Operators / Compound Assignment — 4 tests

| Test | Issue |
|------|-------|
| tests::basic::test_native_proto_operators | `a / b` → should be `@divTrunc(a, b)`; missing `+`, `-`, `*` binary ops |
| tests::basic::test_native_proto_compound_assignment | `a = b` for `**=` → should be `a = std.math.pow(...)`; `and=`/`or=` not expanded |
| tests::basic::test_native_proto_void_operator | `== "2"` → should be `.strictEq(JsAny.from("2"))` |
| tests::basic::test_native_proto_if_else | Return type `anytype` → should be `@TypeOf(x)` |

**Error pattern**: Binary/compound operators not expanded to Zig equivalents

---

## 16. Variable Shadowing — 2 tests

The Lowerer doesn't rename shadowed variables. Zig 0.16 forbids shadowing.

| Test | Emitter emits | Should emit |
|------|---------------|-------------|
| tests::not_implemented_and_fixes::test_shadowing_nested_block | `const x = 20;` | `const x_shadow_0 = 20;` |
| tests::not_implemented_and_fixes::test_shadowing_param_name | `const data = 100;` | `const data_shadow_0 = 100;` |

**Error pattern**: Variable names not renamed to avoid Zig shadowing

---

## 17. Labeled Block — 1 test

| Test | Issue |
|------|-------|
| tests::phase1::test_p1_labeled_block | Label emitted inside block instead of on the block itself |

**Error pattern**: `{ check: if (x > 0) { ... break :check; } }` instead of `check: { if (x > 0) { ... break :check; } }`

---

## 18. Getter/Setter — 2 tests

| Test | Emitter emits | Should emit |
|------|---------------|-------------|
| tests::try_catch_and_closures::test_native_proto_getter | `.{ .x = _fn_expr_0 }` | `.{ .x = 42 }` (eagerly evaluate) |
| tests::try_catch_and_closures::test_native_proto_getter_setter_combined | `.{ .age = _fn_expr_0, .age = _fn_expr_1 }` | `.{ .name = "test", .age = 25 }` |

**Error pattern**: Getter/setter emitted as function expressions instead of direct values

---

## 19. Delete Operator — 1 test

| Test | Emitter emits | Should emit |
|------|---------------|-------------|
| tests::basic::test_native_proto_delete_operator | `_ = ;` | `_ = blk_0: { _ = obj.deleteKey("name"); break :blk_0 true; };` |

**Error pattern**: Empty expression after `_ =`

---

## 20. `in` Operator — 1 test (also in category 8)

| Test | Emitter emits | Should emit |
|------|---------------|-------------|
| tests::phase1::test_p1_in_operator | `"name" == obj` | `obj.contains("name")` |

---

## 21. Optional Property Types — 1 test

| Test | Issue |
|------|-------|
| tests::builtins_basic::test_native_proto_optional_property | Emits `??[]const u8` (double_optional) instead of `?[]const u8` |

---

## 22. BigInt — 1 test

| Test | Emitter emits | Should emit |
|------|---------------|-------------|
| tests::not_implemented_and_fixes::test_bigint_constructor | `js_number.bigIntConstructor(123)` | `js_bigint.JsBigInt.fromI64(alloc, 123) catch @panic("OOM")` |

---

## 23. Ternary-Concat Format Specifier — 1 test

| Test | Emitter emits | Should emit |
|------|---------------|-------------|
| tests::not_implemented_and_fixes::test_native_proto_ternary_concat_parens | `"value: {}", .{if (x>5)...}` | `"value: {s}", .{(if (x>5)...)}` |

**Error pattern**: Missing `{s}` format specifier for string ternary

---

## 24. Return Type `anytype` vs Specific — cross-cutting issue

Many tests above also have the issue that the Emitter emits `-> anytype` or `-> !anytype`
as a return type, while the Codegen emits a specific type like `@TypeOf(x)`, `@TypeOf(obj)`,
or `!@TypeOf(val)`. Zig rejects `anytype` as an explicit return type.

Affected tests (partial list):
- tests::basic::test_native_proto_if_else
- tests::phase1::test_p1_object_assign
- tests::phase1::test_p1_object_freeze
- tests::phase1::test_p1_object_from_entries
- tests::phase1::test_p1_in_operator
- tests::try_catch_and_closures::test_native_proto_throw_bare
- tests::advanced_builtins::test_p7_object_get_own_property_descriptor
- tests::phase1::test_p1_spread_* (various)

**Error pattern**: `error: expected return type expression, found 'anytype'`

---

## 25. Top-level `var` Error Format — 1 test

| Test | Emitter emits | Should emit |
|------|---------------|-------------|
| tests::basic::test_native_proto_toplevel_var_error | `@compileError("toplevel only allows 'const', not 'y'");` | `// error: toplevel only allows 'const', not 'y'` (comment, not @compileError) |

---

## 26. Await/Async — 1 test

| Test | Issue |
|------|-------|
| tests::builtins_basic::test_native_proto_await | Missing `io` param, missing `io.async()`/`_t0.await(io)` pattern |

**Error pattern**: Emits `try double(x)` instead of `io.async(double, .{io, x})` / `try _t0.await(io)`

---

## Summary

| # | Category | Count | Priority |
|---|----------|-------|----------|
| 1 | Destructuring | 8 | P1 |
| 2 | Builtin Method Inlining | 29 | P1 |
| 3 | Callback/Arrow Inlining | 12 | P0 (generates invalid Zig) |
| 4 | RegExp Routing | 8 (+6 overlap) | P1 |
| 5 | Symbol Name Mapping | 6 | P2 |
| 6 | Optional Chaining | 4 | P1 |
| 7 | Exponentiation (`**`) | 4 | P2 |
| 8 | Spread/Rest | 7 | P1 |
| 9 | Missing @compileError (instanceof/in/eval/static) | 5 | P1 |
| 10 | Nested Function Hoisting/Closures | 5 | P1 |
| 11 | Object Builtin Routing | 6 | P2 |
| 12 | Date Constructor Routing | 7 | P2 |
| 13 | TypedArray | 9 | P2 |
| 14 | Map/Set | 3 | P2 |
| 15 | Operators/Compound Assignment | 4 | P2 |
| 16 | Variable Shadowing | 2 | P2 |
| 17 | Labeled Block | 1 | P2 |
| 18 | Getter/Setter | 2 | P2 |
| 19 | Delete Operator | 1 | P2 |
| 20 | `in` Operator | 1 | P2 |
| 21 | Optional Property Types (double `?`) | 1 | P2 |
| 22 | BigInt | 1 | P2 |
| 23 | Ternary-Concat Format | 1 | P2 |
| 24 | Return Type `anytype` | ~10 (cross-cutting) | P1 |
| 25 | Top-level var Error Format | 1 | P3 |
| 26 | Await/Async | 1 | P2 |

**Note**: Some tests appear in multiple categories because they fail for more than one reason.
The total unique test count is 145 (not the sum of category counts).

> AI生成