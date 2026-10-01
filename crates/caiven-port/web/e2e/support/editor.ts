import type { Locator } from '@playwright/test';

/** The remix editor is CodeMirror, not a textarea, so it has no `value`. */
export const editorText = (editor: Locator) =>
  editor.evaluate((node) => [...node.querySelectorAll('.cm-line')].map((line) => line.textContent).join('\n'));

/** Pastes over the whole source: typed or `fill`ed newlines become an extra Enter under mobile emulation. */
export async function fillEditor(editor: Locator, text: string) {
  await editor.click();
  await editor.press('ControlOrMeta+a');
  await editor.evaluate((node, value) => {
    const data = new DataTransfer();
    data.setData('text/plain', value);
    node.dispatchEvent(new ClipboardEvent('paste', { clipboardData: data, bubbles: true, cancelable: true }));
  }, text);
}
