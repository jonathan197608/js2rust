#!/usr/bin/env python3
"""Filter js2rust.toml and main.rs to only include pass fragments."""
import json
import re

PROJECT_DIR = r"C:\Users\18988\RustroverProjects\js2rust\examples\mdn-test-project"

# Read pass fragments
with open(f"{PROJECT_DIR}/pass_fragments.json", "r") as f:
    data = json.load(f)

pass_names = set()
for category in ["statements", "expressions", "builtins"]:
    for name in data[category]:
        pass_names.add(name)

print(f"Pass fragments: {len(pass_names)}")

# --- Filter js2rust.toml ---
toml_path = f"{PROJECT_DIR}/js2rust.toml"
with open(toml_path, "r", encoding="utf-8") as f:
    toml_content = f.read()

# Extract all fragment entries
# Format: "js_src/test_builtins_frag_0.js",
entries = re.findall(r'"(js_src/test_\w+_frag_\d+\.js)"', toml_content)
print(f"Total toml entries: {len(entries)}")

# Filter to only pass fragments
kept_entries = []
removed_count = 0
for entry in entries:
    # Extract fragment name: js_src/test_builtins_frag_0.js -> test_builtins_frag_0
    frag_name = entry.replace("js_src/", "").replace(".js", "")
    if frag_name in pass_names:
        kept_entries.append(entry)
    else:
        removed_count += 1

print(f"Kept: {len(kept_entries)}, Removed: {removed_count}")

# Build new additional_js_files section
lines = []
lines.append('[project]')
lines.append('js_file = "js_src/app.js"')
lines.append('additional_js_files = [')
for i, entry in enumerate(kept_entries):
    comma = "," if i < len(kept_entries) - 1 else ""
    lines.append(f'    "{entry.replace("js_src/", "")}"{comma}')
lines.append(']')
lines.append('')

new_toml = "\n".join(lines)
with open(toml_path, "w", encoding="utf-8") as f:
    f.write(new_toml)

print(f"Written new js2rust.toml with {len(kept_entries)} entries")

# --- Filter main.rs ---
mainrs_path = f"{PROJECT_DIR}/src/main.rs"
with open(mainrs_path, "r", encoding="utf-8") as f:
    main_content = f.read()

# Extract all match arms
# Format: "test_builtins_frag_0" => { testBuiltins_frag_0_app(); },
arm_pattern = r'"(test_\w+_frag_\d+)"\s*=>\s*\{\s*(\w+)\(\)\s*\},?'
arms = re.findall(arm_pattern, main_content)
print(f"\nTotal main.rs match arms: {len(arms)}")

kept_arms = []
removed_arms = 0
for frag_name, func_name in arms:
    if frag_name in pass_names:
        kept_arms.append((frag_name, func_name))
    else:
        removed_arms += 1

print(f"Kept: {len(kept_arms)}, Removed: {removed_arms}")

# Build new main.rs
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
for frag_name, func_name in kept_arms:
    body_lines.append(f'        "{frag_name}" => {{ {func_name}(); }},')

new_main = header + "\n".join(body_lines) + "\n" + footer
with open(mainrs_path, "w", encoding="utf-8") as f:
    f.write(new_main)

print(f"Written new main.rs with {len(kept_arms)} match arms")
