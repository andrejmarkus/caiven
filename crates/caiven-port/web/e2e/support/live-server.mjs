import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawn } from 'node:child_process';

const dataDir = await mkdtemp(join(tmpdir(), 'caiven-port-e2e-'));
const repository = resolve('../../..');
const child = spawn('cargo', [
  'run', '--locked', '-p', 'caiven-port', '--',
  '--address', '127.0.0.1', '--port', '1431',
  '--data-dir', dataDir,
  '--web-dir', resolve('dist'),
  '--base-url', 'http://localhost:1430',
], {
  cwd: repository,
  stdio: 'inherit',
  env: {
    ...process.env,
    RUST_LOG: 'warn',
    HTTP_PROXY: 'http://127.0.0.1:9',
    HTTPS_PROXY: 'http://127.0.0.1:9',
    NO_PROXY: '127.0.0.1,localhost',
  },
});

let closing = false;
async function close(signal = 'SIGTERM') {
  if (closing) return;
  closing = true;
  if (child.exitCode === null) child.kill(signal);
  await rm(dataDir, { recursive: true, force: true });
}

process.on('SIGINT', () => void close('SIGINT'));
process.on('SIGTERM', () => void close());
process.on('exit', () => { if (child.exitCode === null) child.kill('SIGTERM'); });
child.on('exit', async (code, signal) => {
  await rm(dataDir, { recursive: true, force: true });
  process.exitCode = code ?? (signal ? 1 : 0);
});
