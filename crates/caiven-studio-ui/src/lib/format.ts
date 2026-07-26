/** Presentation helpers for paths and counts shown in Studio chrome. */

/**
 * Shorten an absolute path for display. Keeps the last two segments so a cart
 * stays identifiable without the full machine-specific prefix leaking into the
 * header and status line.
 */
export function tidyPath(path: string, keep = 2): string {
  if (!path) return '';
  const segments = path.split('/').filter(Boolean);
  if (segments.length <= keep) return path;
  return `…/${segments.slice(-keep).join('/')}`;
}

/** `count` with a correctly pluralised noun: 1 file, 2 files. */
export function plural(count: number, singular: string, pluralForm = `${singular}s`): string {
  return `${count} ${count === 1 ? singular : pluralForm}`;
}
