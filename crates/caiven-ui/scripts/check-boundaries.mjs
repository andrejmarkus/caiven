import { readFileSync, readdirSync, statSync } from 'node:fs';
import { dirname, extname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const uiRoot = join(repo, 'crates/caiven-ui');
const apps = [
  { name: 'Port', root: join(repo, 'crates/caiven-port/web'), uiLink: 'file:../../caiven-ui' },
  { name: 'Studio', root: join(repo, 'crates/caiven-studio-ui'), uiLink: 'file:../caiven-ui' },
];
const failures = [];
const sourceExtensions = new Set(['.css', '.js', '.svelte', '.ts']);

function json(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function sourceFiles(root) {
  const files = [];
  for (const entry of readdirSync(root)) {
    const path = join(root, entry);
    if (statSync(path).isDirectory()) files.push(...sourceFiles(path));
    else if (sourceExtensions.has(extname(entry))) files.push(path);
  }
  return files;
}

const uiPackage = json(join(uiRoot, 'package.json'));
const peerVersions = uiPackage.peerDependencies ?? {};
const appPackages = new Map();

for (const app of apps) {
  const pkg = json(join(app.root, 'package.json'));
  appPackages.set(app.name, pkg);
  const versions = { ...(pkg.devDependencies ?? {}), ...(pkg.dependencies ?? {}) };
  const localUi = join(app.root, 'src/lib/components/ui');
  try {
    if (statSync(localUi).isDirectory()) failures.push(`${app.name}: local shadcn component tree exists at ${localUi}`);
  } catch {
    // Expected: shared package owns generated primitives.
  }

  if (pkg.dependencies?.['@caiven/ui'] !== app.uiLink) {
    failures.push(`${app.name}: @caiven/ui must be ${app.uiLink}`);
  }

  const css = readFileSync(join(app.root, 'src/app.css'), 'utf8');
  if (!css.includes("@import '@caiven/ui/theme.css';")) failures.push(`${app.name}: app.css must import shared theme`);
  if (!css.includes('@source "../node_modules/@caiven/ui/src";')) failures.push(`${app.name}: app.css must scan shared UI source`);
  if (/--(?:background|foreground|primary|secondary|muted|accent|destructive|border|input|ring|radius)\s*:/.test(css)) {
    failures.push(`${app.name}: semantic theme tokens must stay in @caiven/ui/theme.css`);
  }

  for (const file of sourceFiles(join(app.root, 'src'))) {
    const source = readFileSync(file, 'utf8');
    if (source.includes('$lib/components/ui')) failures.push(`${app.name}: legacy local UI import in ${file}`);
    if (/from\s+['"](?:bits-ui|shadcn-svelte)['"]/.test(source)) failures.push(`${app.name}: import shared @caiven/ui wrapper instead of headless dependency in ${file}`);
  }

  for (const [dependency, peerVersion] of Object.entries(peerVersions)) {
    if (!(dependency in versions)) failures.push(`${app.name}: missing shared UI peer ${dependency}`);
    else if (!peerVersion.startsWith('^') && versions[dependency] !== peerVersion) {
      failures.push(`${app.name}: ${dependency} must match shared version ${peerVersion}, found ${versions[dependency]}`);
    }
  }
}

const portVersions = { ...appPackages.get('Port').devDependencies, ...appPackages.get('Port').dependencies };
const studioVersions = { ...appPackages.get('Studio').devDependencies, ...appPackages.get('Studio').dependencies };
for (const dependency of Object.keys(peerVersions)) {
  if (portVersions[dependency] !== studioVersions[dependency]) {
    failures.push(`Port/Studio dependency drift: ${dependency} is ${portVersions[dependency]} vs ${studioVersions[dependency]}`);
  }
}

if (failures.length) {
  console.error(`Shared UI boundary check failed:\n- ${failures.join('\n- ')}`);
  process.exit(1);
}

console.log('Shared UI boundaries and dependency parity verified.');
