// Pure helpers behind Quick Remix. Every affordance here edits the real Lua
// text; nothing is stored beside it.

const CONSTANT = /^(\s*local\s+)([A-Z][A-Z0-9_]*)(\s*=\s*)(-?\d+(?:\.\d+)?)(\s*(?:--.*)?)$/;
const TRY = /--\s*try\s+(-?\d+(?:\.\d+)?)\b/i;

/**
 * Top-level numeric constants (`local SPEED = 2`) — the obvious first thing
 * to change in a small cart. `suggestion` is the author's `-- try N` value.
 * @param {string} source
 * @returns {{ name: string, value: string, line: number, suggestion: string | null }[]}
 */
export function findConstants(source, limit = 6) {
  const out = [];
  const lines = source.split('\n');
  for (let i = 0; i < lines.length && out.length < limit; i++) {
    const m = CONSTANT.exec(lines[i]);
    if (m) out.push({ name: m[2], value: m[4], line: i + 1, suggestion: TRY.exec(m[5])?.[1] ?? null });
  }
  return out;
}

/**
 * Rewrites one constant's value in place, keeping the rest of the line.
 * @param {string} source @param {number} line 1-based @param {string} value
 */
export function setConstant(source, line, value) {
  if (!/^-?\d+(?:\.\d+)?$/.test(value)) return source;
  const lines = source.split('\n');
  const m = CONSTANT.exec(lines[line - 1] ?? '');
  if (!m) return source;
  lines[line - 1] = `${m[1]}${m[2]}${m[3]}${value}${m[5]}`;
  return lines.join('\n');
}

/**
 * Pulls the cart line out of a VM error (`cart:12: ...`, also inside
 * `[string "cart"]:12:` or a traceback).
 * @param {string} message
 * @returns {{ line: number | null, detail: string }}
 */
export function parseLuaError(message) {
  const m = /(?:\[string "cart"\]|\bcart):(\d+):\s*([^\n]*)/.exec(message);
  if (!m) return { line: null, detail: message.split('\n')[0].trim() };
  return { line: Number(m[1]), detail: m[2].trim() };
}

/** @type {Array<[RegExp, (m: RegExpExecArray) => string]>} */
const HINTS = [
  [/attempt to call a nil value \((?:global|field|method) '([^']+)'\)/, (m) => `\`${m[1]}\` doesn't exist. Check the spelling, or define it before this line runs.`],
  [/attempt to (?:perform arithmetic|compare|concatenate).*nil value(?: \((?:global|local|field) '([^']+)'\))?/, (m) => m[1] ? `\`${m[1]}\` has no value yet (it's nil), so it can't be used in math here.` : 'A value used here is nil. Something it depends on was never set.'],
  [/attempt to index a nil value(?: \((?:global|local|field) '([^']+)'\))?/, (m) => m[1] ? `\`${m[1]}\` is nil, so you can't read fields from it.` : 'You\'re reading a field from something that is nil.'],
  [/'end' expected/, () => 'A `function`, `if`, `for` or `while` block is missing its closing `end`.'],
  [/'then' expected/, () => 'An `if` condition needs `then` after it.'],
  [/'do' expected/, () => 'A `for` or `while` loop needs `do` after it.'],
  [/'\)' expected/, () => 'A `(` was opened and never closed.'],
  [/'=' expected/, () => 'Lua expected an assignment here. Look for a typo in a name.'],
  [/unexpected symbol|syntax error/, () => 'Lua couldn\'t read this line. Look for a typo or a missing operator.'],
  [/unfinished string/, () => 'A string is missing its closing quote.'],
  [/budget|too long|watchdog/i, () => 'One frame ran too long. Maybe a loop that never ends?'],
];

/** Plain-language gloss for the commonest beginner errors, or null. @param {string} detail */
export function errorHint(detail) {
  for (const [re, hint] of HINTS) {
    const m = re.exec(detail);
    if (m) return hint(m);
  }
  return null;
}
