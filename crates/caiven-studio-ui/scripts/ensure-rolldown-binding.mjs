import { execFileSync } from 'node:child_process';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);

try {
  require('rolldown');
  process.exit(0);
} catch (error) {
  const packages = {
    'linux-x64': '@rolldown/binding-linux-x64-gnu@1.1.5',
    'linux-arm64': '@rolldown/binding-linux-arm64-gnu@1.1.5',
    'darwin-x64': '@rolldown/binding-darwin-x64@1.1.5',
    'darwin-arm64': '@rolldown/binding-darwin-arm64@1.1.5',
    'win32-x64': '@rolldown/binding-win32-x64-msvc@1.1.5',
    'win32-arm64': '@rolldown/binding-win32-arm64-msvc@1.1.5',
  };

  const key = `${process.platform}-${process.arch}`;
  const binding = packages[key];
  if (!binding) throw error;

  console.log(`Rolldown native binding is missing; installing ${binding}`);
  execFileSync('npm', ['install', '--no-save', '--ignore-scripts=false', binding], {
    cwd: new URL('..', import.meta.url),
    stdio: 'inherit',
    shell: process.platform === 'win32',
  });

  require('rolldown');
}
