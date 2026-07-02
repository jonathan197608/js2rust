#!/usr/bin/env python3
"""Generate main.rs match arms from pass_fragments.json."""
import json

PROJECT_DIR = r"C:\Users\18988\RustroverProjects\js2rust\examples\mdn-test-project"

with open(f"{PROJECT_DIR}/pass_fragments.json", "r") as f:
    data = json.load(f)

pass_names = []
for category in ["statements", "expressions", "builtins"]:
    for name in data[category]:
        pass_names.append(name)

print(f"Pass fragments: {len(pass_names)}")

# Generate function name: test_builtins_frag_0 -> testBuiltins_frag_0_app
def to_func_name(frag_name):
    # test_builtins_frag_0 -> testBuiltins_frag_0_app
    # test_expressions_frag_100 -> testExpressions_frag_100_app
    # test_statements_frag_0 -> testStatements_frag_0_app
    parts = frag_name.split("_", 2)  # ["test", "builtins", "frag_0"]
    prefix = parts[0]  # "test"
    category = parts[1]  # "builtins"
    rest = parts[2]  # "frag_0"
    # Capitalize category
    cap_category = category[0].upper() + category[1:]
    return f"{prefix}{cap_category}_{rest}_app"

header = """// src/main.rs -- CLI dispatcher: ./mdn-test-project <fragment_name>
use std::env;
use js2rust_bridge::js2rust_bridge;

js2rust_bridge!();

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <fragment_name>", args[0]);
        std::process::exit(1);
    }
    let frag = &args[1];
    js2rust_init();
    match frag.as_str() {
"""

footer = """        _ => {
            eprintln!("Unknown fragment: {}", frag);
            std::process::exit(1);
        }
    }
    js2rust_deinit();
}
"""

body_lines = []
for frag_name in pass_names:
    func_name = to_func_name(frag_name)
    body_lines.append(f'        "{frag_name}" => {{ {func_name}(); }},')

new_main = header + "\n".join(body_lines) + "\n" + footer

with open(f"{PROJECT_DIR}/src/main.rs", "w", encoding="utf-8") as f:
    f.write(new_main)

print(f"Written main.rs with {len(pass_names)} match arms")

# Verify a few examples
for name in pass_names[:5]:
    print(f"  {name} -> {to_func_name(name)}")
