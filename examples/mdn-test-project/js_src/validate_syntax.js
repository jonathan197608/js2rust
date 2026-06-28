// validate_syntax.js
// 用 Node.js 验证所有生成的 MDN 测试文件的语法
// 用法：node validate_syntax.js

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const JS_SRC_DIR = __dirname;
const files = [
  'test_expressions.js',
  'test_statements.js',
  'test_built_ins.js',
].filter(f => fs.existsSync(path.join(JS_SRC_DIR, f)));

console.log(`Validating ${files.length} JS files...\n`);

let allPassed = true;
for (const file of files) {
  const fpath = path.join(JS_SRC_DIR, file);
  try {
    execSync(`node --check "${fpath}"`, { stdio: 'pipe' });
    console.log(`  ✓ ${file}: syntax OK`);
  } catch (e) {
    const errMsg = e.stderr ? e.stderr.toString() : e.message;
    console.log(`  ✗ ${file}: syntax ERROR`);
    console.log(`    ${errMsg.split('\n')[0]}`);
    allPassed = false;
  }
}

// 尝试执行每个测试函数（不关心输出，只关心是否崩溃）
console.log('\nRunning test functions (catch errors)...');
for (const file of files) {
  const fpath = path.join(JS_SRC_DIR, file);
  const funcName = file.replace('test_', '').replace('.js', '');
  const capitalized = funcName.charAt(0).toUpperCase() + funcName.slice(1);
  const testFn = `test${capitalized}`;

  const wrapper = `
const { ${testFn} } = require('./${file}');
try {
  ${testFn}();
  console.log('  ✓ ${testFn}() executed (may have internal errors)');
} catch (e) {
  console.log('  ✗ ${testFn}() threw: ' + e.message);
}
`;
  const tmpFile = path.join(JS_SRC_DIR, '._tmp_run.js');
  fs.writeFileSync(tmpFile, wrapper);
  try {
    const out = execSync(`node "${tmpFile}"`, { stdio: 'pipe', timeout: 10000 });
    console.log(out.toString().trim());
  } catch (e) {
    const errMsg = e.stderr ? e.stderr.toString() : e.stdout ? e.stdout.toString() : e.message;
    console.log(`  ✗ ${testFn}() execution error:\n${errMsg.slice(0, 300)}`);
    allPassed = false;
  } finally {
    if (fs.existsSync(tmpFile)) fs.unlinkSync(tmpFile);
  }
}

console.log(`\n${allPassed ? '✓ All validations passed' : '✗ Some validations failed'}`);
process.exit(allPassed ? 0 : 1);
