import type { ApiEntry } from '../types';

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

/** Project symbols visible from `text`: every module's globals plus `alias.member` for its requires. */
export function projectEntries(modules: ProjectModule[], text: string): ApiEntry[] {
  const scans = new Map(modules.map((module) => [module.key, scanModule(module.key, module.text)]));
  const entries = [...scans.values()].flatMap((scan) => scan.globals);
  for (const [alias, key] of requireAliases(text)) {
    for (const entry of scans.get(key)?.exports ?? []) entries.push({ ...entry, name: `${alias}.${entry.name}` });
  }
  return entries;
}
