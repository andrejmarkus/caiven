export interface ApiEntry {
  name: string;
  params: { name: string; ty: string }[];
  returns: string;
  doc: string;
  category: string;
}

export interface PreludeModule {
  name: string;
  /** Conventional local for the module's table, e.g. `Camera`. */
  export: string;
}

export interface ProjectModule {
  /** `require` key, e.g. `ui.hud` for `ui/hud.lua`. */
  key: string;
  text: string;
  /** The cart's entry file: scanned for globals, never offered to `require`. */
  entry: boolean;
}

export interface ModuleScan {
  /** Globals the module defines, including members of global tables (`Enemy.new`). */
  globals: ApiEntry[];
  /** Members of the table the module returns, named without the table prefix. */
  exports: ApiEntry[];
}

const NAME = '[A-Za-z_]\\w*';

/** `ui/hud.lua` → `ui.hud`, the same key `caiven_cart::module_key` builds. */
export function moduleKey(fileName: string): string {
  return fileName.replace(/\\/g, '/').replace(/\.lua$/, '').split('/').join('.');
}

// ponytail: lexical, column-0 definitions only; a real Lua parser if nested/indented definitions matter.
/** What a module defines at top level, with the `--` comment above each definition as its doc. */
export function scanModule(key: string, text: string): ModuleScan {
  const returned = [...text.matchAll(new RegExp(`^return\\s+(${NAME})\\s*$`, 'gm'))].at(-1)?.[1];
  const locals = new Set([...text.matchAll(new RegExp(`^local\\s+(${NAME})`, 'gm'))].map((match) => match[1]));
  const scan: ModuleScan = { globals: [], exports: [] };
  const seen = new Set<string>();
  let doc: string[] = [];
  for (const line of text.split(/\r?\n/)) {
    const comment = line.match(/^--+\s?(.*)$/);
    if (comment) {
      doc.push(comment[1].trim());
      continue;
    }
    const definition = line.match(new RegExp(`^function\\s+(${NAME})(?:[.:](${NAME}))?\\s*\\(([^)]*)\\)`))
      ?? line.match(new RegExp(`^(${NAME})(?:\\.(${NAME}))?\\s*=(?!=)\\s*(?:function\\s*\\(([^)]*)\\))?`));
    if (definition) {
      const [, owner, member, params] = definition;
      const entry = (name: string): ApiEntry => ({
        name,
        params: (params ?? '').split(',').map((param) => param.trim()).filter(Boolean).map((param) => ({ name: param, ty: '' })),
        returns: '',
        doc: doc.join(' ') || `Defined in ${key}.`,
        category: params === undefined ? 'Project value' : 'Project',
      });
      const target = member && owner === returned ? scan.exports : scan.globals;
      const name = member && owner === returned ? member : member ? `${owner}.${member}` : owner;
      const visible = owner === returned ? Boolean(member) : !locals.has(owner);
      if (visible && !seen.has(`${target === scan.exports}:${name}`)) {
        seen.add(`${target === scan.exports}:${name}`);
        target.push(entry(name));
      }
    }
    doc = [];
  }
  return scan;
}

/** `local hud = require("ui.hud")` → `hud` ↦ `ui.hud`. */
export function requireAliases(text: string): Map<string, string> {
  const aliases = new Map<string, string>();
  const pattern = new RegExp(`local\\s+(${NAME})\\s*=\\s*require\\s*\\(?\\s*["']([\\w./]+)["']`, 'g');
  for (const match of text.matchAll(pattern)) aliases.set(match[1], match[2].replace(/\//g, '.'));
  return aliases;
}

/** The module name under `column` when it sits inside a `require("…")` string. */
export function requireNameAt(line: string, column: number): { from: number; to: number; name: string } | null {
  for (const match of line.matchAll(/require\s*\(?\s*["']([\w./]*)/g)) {
    const from = (match.index ?? 0) + match[0].length - match[1].length;
    const to = from + match[1].length;
    if (column >= from && column <= to) return { from, to, name: match[1].replace(/\//g, '.') };
  }
  return null;
}

/** Conventional local for a project module's table: `ui.hud` → `hud`. */
export function localName(key: string): string {
  return key.split('.').at(-1) ?? key;
}

/** Whether `text` declares `name` as a local (`local Camera = …`, `local a, Camera = …`). */
export function declaresLocal(text: string, name: string): boolean {
  return new RegExp(`\\blocal\\s+(?:${NAME}\\s*,\\s*)*${name}\\b`).test(text);
}

/** Console API minus members of built-in modules `text` never binds under their conventional local. */
export function availableApi(
  api: ApiEntry[], modules: { name: string; export: string }[], text: string, shadowed: string[] = [],
): ApiEntry[] {
  const hidden = modules
    .filter((module) => shadowed.includes(module.name) || !declaresLocal(text, module.export))
    .map((module) => module.export);
  return api.filter((entry) => !hidden.some((name) => new RegExp(`^${name}(?:[.:]|$)`).test(entry.name)));
}

/**
 * Built-in API entries under the alias a file binds a module to:
 * `local tw = require "tween"` turns `tween.new` into `tw.new`.
 */
export function builtinAliasEntries(
  api: ApiEntry[], modules: { name: string; export: string }[], text: string, shadowed: string[] = [],
): ApiEntry[] {
  const entries: ApiEntry[] = [];
  for (const [alias, key] of requireAliases(text)) {
    const module = modules.find((candidate) => candidate.name === key);
    if (!module || alias === module.export || shadowed.includes(key)) continue;
    for (const entry of api) {
      const match = entry.name.match(new RegExp(`^${module.export}([.:].*)$`));
      if (match) entries.push({ ...entry, name: `${alias}${match[1]}` });
    }
  }
  return entries;
}

/** Every module `text` requires, with or without a local. */
export function requiredKeys(text: string): Set<string> {
  return new Set([...text.matchAll(/require\s*\(?\s*["']([\w./]+)["']/g)].map((match) => match[1].replace(/\//g, '.')));
}

/** Project symbols visible from `text` (module `self`): its own globals, plus globals and `alias.member` of what it requires. */
export function projectEntries(modules: ProjectModule[], text: string, self = ''): ApiEntry[] {
  const scans = new Map(modules.map((module) => [module.key, scanModule(module.key, module.text)]));
  const entries = scanModule(self, text).globals;
  for (const key of requiredKeys(text)) if (key !== self) entries.push(...(scans.get(key)?.globals ?? []));
  for (const [alias, key] of requireAliases(text)) {
    for (const entry of scans.get(key)?.exports ?? []) entries.push({ ...entry, name: `${alias}.${entry.name}` });
  }
  return entries;
}

const LUA_KEYWORDS = new Set([
  'and', 'break', 'do', 'else', 'elseif', 'end', 'false', 'for', 'function', 'goto', 'if', 'in',
  'local', 'nil', 'not', 'or', 'repeat', 'return', 'then', 'true', 'until', 'while',
]);

/** The watch path (`player.x`, `items[1]`) ending at the part of `text` under `column`, for a hover. */
export function watchPathAt(text: string, column: number): { from: number; to: number; path: string } | null {
  for (const match of text.matchAll(/[A-Za-z_]\w*(?:\.[A-Za-z_]\w*|\[\d+\])*/g)) {
    const start = match.index ?? 0;
    if (column < start || column >= start + match[0].length) continue;
    // A method name or a field of a call result has no path of its own.
    if (start > 0 && /[.:]/.test(text[start - 1])) return null;
    let end = 0;
    for (const part of match[0].matchAll(/[A-Za-z_]\w*|\.[A-Za-z_]\w*|\[\d+\]/g)) {
      end = (part.index ?? 0) + part[0].length;
      if (start + end > column) break;
    }
    const path = match[0].slice(0, end);
    return LUA_KEYWORDS.has(path) ? null : { from: start, to: start + end, path };
  }
  return null;
}

export function sourceOffset(source: string, line: number, column = 1): number {
  const lines = source.split('\n');
  const targetLine = Math.max(1, Math.min(lines.length, Math.trunc(line) || 1));
  let offset = 0;
  for (let index = 0; index < targetLine - 1; index += 1) offset += lines[index].length + 1;
  const lineText = lines[targetLine - 1] ?? '';
  const targetColumn = Math.max(1, Math.min(lineText.length + 1, Math.trunc(column) || 1));
  return offset + targetColumn - 1;
}
