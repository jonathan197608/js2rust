// run_node.js -- Run a single fragment with Node.js
// Usage: node run_node.js <fragment_name>
// Reads the JS file from js_src/<fragment_name>.js, imports it, calls the function
import { readFile } from 'fs/promises';

const fragName = process.argv[2];
if (!fragName) {
    console.error('Usage: node run_node.js <fragment_name>');
    process.exit(1);
}

const jsPath = `js_src/${fragName}.js`;
try {
    const mod = await import(`./${jsPath}`);
    // Find the exported function - it should match the pattern test<Category>_frag_<N>
    const funcName = fragName.replace(/^test_/, 'test')
        .replace(/^test(\w)_/, (m, c) => 'test' + c.toUpperCase() + '_');
    // Actually, the function name is: test_builtins_frag_0 -> testBuiltins_frag_0
    const parts = fragName.split('_', 2);
    const category = parts[1];
    const rest = fragName.substring(fragName.indexOf('_', 5) + 1);
    const expectedFunc = 'test' + category[0].toUpperCase() + category.substring(1) + '_' + rest;

    if (mod[expectedFunc]) {
        mod[expectedFunc]();
    } else {
        // Try to find any exported function
        const keys = Object.keys(mod);
        if (keys.length > 0) {
            mod[keys[0]]();
        } else {
            console.error(`No exported function found in ${jsPath}`);
            process.exit(1);
        }
    }
} catch (e) {
    console.error(`Error running ${fragName}: ${e.message}`);
    process.exit(1);
}
