#!/usr/bin/env python3
"""Fix js2rust.toml to add js_src/ prefix."""
import json

PROJECT_DIR = r"C:\Users\18988\RustroverProjects\js2rust\examples\mdn-test-project"

with open(f"{PROJECT_DIR}/pass_fragments.json", "r") as f:
    data = json.load(f)

pass_names = []
for category in ["statements", "expressions", "builtins"]:
    for name in data[category]:
        pass_names.append(name)

lines = []
lines.append('[project]')
lines.append('js_file = "js_src/app.js"')
lines.append('additional_js_files = [')
for i, name in enumerate(pass_names):
    comma = "," if i < len(pass_names) - 1 else ""
    lines.append(f'    "js_src/{name}.js"{comma}')
lines.append(']')
lines.append('')

with open(f"{PROJECT_DIR}/js2rust.toml", "w", encoding="utf-8") as f:
    f.write("\n".join(lines))

print(f"Fixed js2rust.toml with {len(pass_names)} entries (with js_src/ prefix)")
