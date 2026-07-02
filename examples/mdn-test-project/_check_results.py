import json
with open('comparison_results.json') as f:
    data = json.load(f)
print('=== SUMMARY ===')
print(f"total: {data['total']}")
print(f"matches: {data['matches']}")
print(f"mismatches: {data['mismatches']}")
print(f"errors: {data['errors']}")
print()
print('=== MISMATCHES ===')
for f, n, z in data['mismatch_list']:
    print(f'{f}:')
    print(f'  node: {repr(n[:120])}')
    print(f'  zig:  {repr(z[:120])}')
    print()
