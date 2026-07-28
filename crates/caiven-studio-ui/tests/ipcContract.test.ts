import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');

function read(path: string): string {
  return readFileSync(resolve(root, path), 'utf8');
}

function uniqueSorted(values: Iterable<string>): string[] {
  return [...new Set(values)].sort();
}

function frontendCommands(source: string): string[] {
  return uniqueSorted([...source.matchAll(/\binvoke(?:<[^;()]+>)?\(\s*['"]([^'"]+)['"]/g)].map((match) => match[1]));
}

function registeredCommands(source: string): string[] {
  const block = source.match(/generate_handler!\[([\s\S]*?)\]\)/)?.[1];
  assert.ok(block, 'Rust Tauri generate_handler! registry not found');
  return uniqueSorted(block.split(',').map((entry) => entry.trim().split('::').at(-1)!).filter(Boolean));
}

function mockedCommands(source: string): string[] {
  return uniqueSorted(
    [...source.matchAll(/command === ['"]([^'"]+)['"]/g)]
      .map((match) => match[1])
      .filter((command) => !command.startsWith('plugin:')),
  );
}

test('frontend IPC commands match registered Rust handlers and strict E2E mocks', () => {
  const frontend = frontendCommands(read('src/lib/ipc.ts'));
  const registered = registeredCommands(read('../caiven-studio/src/tauri_app.rs'));
  const mocked = mockedCommands(read('e2e/fixtures.ts'));

  assert.deepEqual(registered, frontend, 'Rust command registry drifted from frontend IPC client');
  assert.deepEqual(mocked, frontend, 'E2E mock dispatcher drifted from frontend IPC client');
});
