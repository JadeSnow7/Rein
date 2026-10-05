import { existsSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { cargoTargetDir } from '../build-artifacts.mjs';

const root = fileURLToPath(new URL('../..', import.meta.url));
const out = join(root, 'contracts/runtime/schemas');
const check = process.argv.includes('--check');
if (process.argv.slice(2).some(arg => arg !== '--check')) throw new Error('usage: node scripts/runtime/check-contracts.mjs [--check]');
if (!check) mkdirSync(out, { recursive: true });
function run(command, args) {
  const targetDir = cargoTargetDir(root, join(root, 'target'));
  const result = spawnSync(command, args, {
    cwd: root,
    stdio: 'inherit',
    env: { ...process.env, CARGO_TARGET_DIR: targetDir },
  });
  if (result.status !== 0) process.exit(result.status ?? 1);
}
run('cargo', ['run', '--quiet', '--locked', '-p', 'rein-runtime', '--', 'schema', '--output', out, ...(check ? ['--check'] : [])]);
if (!check) {
  for (const name of ['session', 'observation', 'intent', 'step-result']) {
    run('node', ['scripts/runtime/schema-to-ts.mjs', '--input', join(out, `${name}.json`), '--output', join(out, `${name}.ts`)]);
  }
}
for (const name of ['session', 'observation', 'intent', 'step-result']) {
  if (!existsSync(join(out, `${name}.json`))) throw new Error(`missing ${name}.json`);
  if (!existsSync(join(out, `${name}.ts`))) throw new Error(`missing ${name}.ts`);
  if (check) run('node', ['scripts/runtime/schema-to-ts.mjs', '--input', join(out, `${name}.json`), '--output', join(out, `${name}.ts`), '--check']);
}
run(process.execPath, [join(root, 'node_modules/typescript/bin/tsc'), '--strict', '--noEmit', '--skipLibCheck', ...['session', 'observation', 'intent', 'step-result'].map(name => join(out, `${name}.ts`))]);
console.log(`runtime contracts ${check ? 'checked' : 'generated'}`);
