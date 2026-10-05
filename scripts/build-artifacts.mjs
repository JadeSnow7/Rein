import { existsSync, readFileSync, realpathSync } from 'node:fs';
import { homedir } from 'node:os';
import { isAbsolute, join } from 'node:path';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';

function mounted(path) {
  if (path.startsWith('/Volumes/')) {
    const volume = path.split('/').slice(0, 3).join('/');
    if (!execFileSync('/sbin/mount', [], { encoding: 'utf8' }).includes(` on ${volume} (`)) {
      throw new Error(`build artifacts volume is not mounted: ${volume}`);
    }
  }
}

export function cargoTargetDir(root, fallback) {
  if (process.env.CARGO_TARGET_DIR) {
    mounted(process.env.CARGO_TARGET_DIR);
    return process.env.CARGO_TARGET_DIR;
  }
  const config = join(homedir(), '.config/build-artifacts/root');
  const artifactRoot = process.env.BUILD_ARTIFACTS_ROOT || (existsSync(config) ? readFileSync(config, 'utf8').trim() : null);
  if (artifactRoot === null) return fallback;
  if (!artifactRoot || !isAbsolute(artifactRoot)) throw new Error('build artifacts root must be an absolute path');
  mounted(artifactRoot);
  if (!existsSync(artifactRoot)) throw new Error(`build artifacts root is missing: ${artifactRoot}`);
  const identity = createHash('sha256').update(realpathSync(root)).digest('hex').slice(0, 16);
  return join(artifactRoot, `rein-${identity}`, 'cargo');
}
