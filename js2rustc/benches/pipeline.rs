use criterion::{Criterion, criterion_group, criterion_main};

// A realistic JS module covering arrays, objects, closures, control flow.
const BENCH_JS: &str = r#"
function factorial(n) {
    if (n <= 1) { return 1; }
    return n * factorial(n - 1);
}

function fibonacci(n) {
    if (n <= 1) { return n; }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

function sumArray(arr) {
    let total = 0;
    for (let i = 0; i < arr.length; i++) {
        total += arr[i];
    }
    return total;
}

function makeCounter(init) {
    let count = init;
    return () => { count += 1; return count; };
}

function processMap(m) {
    let result = [];
    for (const [k, v] of m) {
        result.push({ key: k, value: v });
    }
    return result;
}

class Point {
    x;
    y;
    constructor(x, y) {
        this.x = x;
        this.y = y;
    }
    distance() {
        return Math.sqrt(this.x * this.x + this.y * this.y);
    }
}

function tryJSON(str) {
    try {
        return JSON.parse(str);
    } catch (e) {
        return null;
    }
}

export { factorial, fibonacci, sumArray, makeCounter, processMap, Point, tryJSON };
"#;

fn bench_parse(c: &mut Criterion) {
    c.bench_function("parse", |b| {
        let alloc = oxc_allocator::Allocator::default();
        b.iter(|| {
            js2rustc::parser::parse(&alloc, BENCH_JS);
        });
    });
}

fn bench_strip_imports(c: &mut Criterion) {
    c.bench_function("strip_imports", |b| {
        b.iter(|| {
            js2rustc::analyzer::strip_imports_extract_exports(BENCH_JS);
        });
    });
}

fn bench_pipeline(c: &mut Criterion) {
    c.bench_function("pipeline_full", |b| {
        let alloc = oxc_allocator::Allocator::default();
        let program = js2rustc::parser::parse(&alloc, BENCH_JS);

        let builtins = js2rustc::builtins::BuiltinRegistry::new();
        let exports = std::collections::HashSet::new();

        b.iter(|| {
            js2rustc::codegen::generate(&program, &builtins, &exports, BENCH_JS, "bench.js");
        });
    });
}

fn bench_codegen(c: &mut Criterion) {
    let alloc = oxc_allocator::Allocator::default();
    let program = js2rustc::parser::parse(&alloc, BENCH_JS);
    let builtins = js2rustc::builtins::BuiltinRegistry::new();
    let exports = std::collections::HashSet::new();

    c.bench_function("codegen", |b| {
        b.iter(|| {
            js2rustc::codegen::generate(&program, &builtins, &exports, BENCH_JS, "bench.js");
        });
    });
}

criterion_group!(benches, bench_parse, bench_strip_imports, bench_pipeline, bench_codegen);
criterion_main!(benches);
