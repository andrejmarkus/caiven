/** Presentation helpers for paths and counts shown in Studio chrome. */

/**
 * Shorten an absolute path for display. Keeps the last two segments so a cart
 * stays identifiable without the full machine-specific prefix leaking into the
 * header and status line.
 */
export function tidyPath(path: string, keep = 2): string {
  if (!path) return '';
  const sep = path.includes('\\') && !path.includes('/') ? '\\' : '/';
  const segments = path.split(/[/\\]/).filter(Boolean);
  if (segments.length <= keep) return path;
  return `…${sep}${segments.slice(-keep).join(sep)}`;
}

/** Last path segment, splitting on both `/` and `\` so Windows paths work. */
export function fileName(path: string): string {
  return path.split(/[/\\]/).filter(Boolean).at(-1) ?? path;
}

/** A cart title as a default file name: no characters Windows rejects, no trailing dot/space, no device name. */
export function safeFileName(title: string): string {
  const name = title.replace(/[<>:"/\\|?*\u0000-\u001f]/g, '_').replace(/[. ]+$/, '').trim();
  if (!name) return 'cart';
  return /^(con|prn|aux|nul|com\d|lpt\d)$/i.test(name) ? `${name}_` : name;
}

const isMac =typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.userAgent);
const MODIFIERS: Record<string, string> = { '⌘': 'Ctrl', '⇧': 'Shift' };

/** A Mac-style label (`⇧⌘Z`) as this platform writes it: `Ctrl+Shift+Z` off Mac. */
export function shortcut(keys: string, mac = isMac): string {
  if (mac) return keys;
  const mods = Object.keys(MODIFIERS).filter((symbol) => keys.includes(symbol)).map((symbol) => MODIFIERS[symbol]);
  return [...mods, [...keys].filter((c) => !(c in MODIFIERS)).join('')].join('+');
}

/** Label for a `KeyboardEvent.code`; `layout` (from `navigator.keyboard.getLayoutMap()`) gives the printed character. */
export function keyLabel(code: string, layout?: ReadonlyMap<string, string>): string {
  const printed = layout?.get(code)?.trim();
  if (printed) return printed.toUpperCase();
  if (code.startsWith('Arrow')) return code.slice(5);
  const single = /^(?:Key|Digit)(.)$/.exec(code);
  if (single) return single[1];
  return code.replace(/(Left|Right)$/, ' $1');
}

/** `count` with a correctly pluralised noun: 1 file, 2 files. */
export function plural(count: number, singular: string, pluralForm = `${singular}s`): string {
  return `${count} ${count === 1 ? singular : pluralForm}`;
}
