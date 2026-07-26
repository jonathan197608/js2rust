// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 30
// Targeting String-variable ambiguous-method dispatch (R31-LOWER-4)
// detect_builtin_call only inspects syntax (StringLiteral vs other), so
// .concat()/.at() on a Str VARIABLE misroutes to ArrayConcat/ArrayAt.
// The lowerer's Str+JsArray fix-up must redirect to JsString, and
// infer_expr_type's Str inline must return Str so decl.rs var-decl
// type agrees with BuiltinCall IR's return_type.

export function testEdge_frag_30() {
    const s = "hello";
    console.log(s.concat(" world"));
    const r1 = s.concat(" world");
    console.log(r1);
    console.log(s.at(2));
    const r2 = s.at(2);
    console.log(r2);
    console.log(s.slice(1, 3));
    const r3 = s.slice(1, 3);
    console.log(r3);
    console.log(s.includes("ell"));
    const r4 = s.includes("ell");
    console.log(r4);
}