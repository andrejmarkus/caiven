<script lang="ts">
  import { onMount } from 'svelte';
  import { Annotation, EditorState, RangeSet, StateEffect, StateField } from '@codemirror/state';
  import {
    EditorView, GutterMarker, drawSelection, dropCursor, gutter, highlightActiveLine,
    highlightActiveLineGutter, highlightSpecialChars, hoverTooltip, keymap, lineNumbers,
    rectangularSelection,
  } from '@codemirror/view';
  import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
  import { autocompletion, completionKeymap, type CompletionContext } from '@codemirror/autocomplete';
  import { bracketMatching, HighlightStyle, syntaxHighlighting, StreamLanguage } from '@codemirror/language';
  import { lintKeymap, setDiagnostics, type Diagnostic as CmDiagnostic } from '@codemirror/lint';
  import { searchKeymap } from '@codemirror/search';
  import { lua } from '@codemirror/legacy-modes/mode/lua';
  import { tags as t } from '@lezer/highlight';
  import type {
    ApiEntry, Breakpoint, Diagnostic, EditorInsertRequest, EditorRevealRequest, PreludeModule,
  } from '../types';
  import { sourceOffset } from '../lib/editorMath';

  // CodeMirror's own `defaultHighlightStyle` assumes a light background —
  // against this editor's dark theme it renders near-black text on black,
  // so tokens need their own dark-aware palette instead.
  const luaHighlightStyle = HighlightStyle.define([
    { tag: t.comment, color: 'var(--color-ink-faint)', fontStyle: 'italic' },
    { tag: t.keyword, color: 'var(--color-ember)', fontWeight: '600' },
    { tag: [t.bool, t.atom, t.null], color: 'var(--color-ember-bright)' },
    { tag: t.number, color: '#d6a8f0' },
    { tag: t.string, color: '#9fd88c' },
    { tag: [t.definition(t.variableName), t.function(t.variableName)], color: 'var(--color-sheen-bright)' },
    { tag: t.propertyName, color: 'var(--color-sheen-bright)' },
    { tag: t.variableName, color: 'var(--color-ink)' },
    { tag: t.operator, color: 'var(--color-ink-dim)' },
    { tag: [t.bracket, t.punctuation], color: 'var(--color-ink-dim)' },
  ]);

  interface Props {
    value: string;
    path: string;
    initialCursor: number;
    api: ApiEntry[];
    preludeModules: PreludeModule[];
    diagnostics: Diagnostic[];
    breakpoints: Breakpoint[];
    insertRequest: EditorInsertRequest | null;
    revealRequest: EditorRevealRequest | null;
    onInsertHandled: (id: number) => void;
    onRevealHandled: (id: number) => void;
    onChange: (value: string) => void;
    onCursor: (source: string, offset: number) => void;
    onToggleBreakpoint: (source: string, line: number) => void;
    onEnableModule: (module: string) => void;
  }

  let {
    value, path, initialCursor, api, preludeModules, diagnostics, breakpoints, insertRequest, revealRequest,
    onInsertHandled, onRevealHandled, onChange, onCursor, onToggleBreakpoint, onEnableModule,
  }: Props = $props();
  let host: HTMLDivElement;
  let view: EditorView | undefined;
  let handledInsert = 0;
  let handledReveal = 0;

  class BreakpointMarker extends GutterMarker {
    toDOM() {
      const node = document.createElement('span');
      node.className = 'cm-breakpoint-dot';
      return node;
    }
  }
  const marker = new BreakpointMarker();
  const externalDocument = Annotation.define<boolean>();
  const setBreakpoints = StateEffect.define<number[]>();
  const breakpointField = StateField.define<RangeSet<GutterMarker>>({
    create: () => RangeSet.empty,
    update(markers, transaction) {
      for (const effect of transaction.effects) {
        if (!effect.is(setBreakpoints)) continue;
        const ranges = effect.value
          .filter((line) => line > 0 && line <= transaction.state.doc.lines)
          .map((line) => marker.range(transaction.state.doc.line(line).from));
        return RangeSet.of(ranges, true);
      }
      return markers.map(transaction.changes);
    },
  });

  function completions(context: CompletionContext) {
    const word = context.matchBefore(/[\w.]*/);
    if (!word || (!context.explicit && word.from === word.to)) return null;
    return {
      from: word.from,
      options: api.map((entry) => ({
        label: entry.name,
        type: 'function',
        detail: `(${entry.params.map((param) => param.name).join(', ')}) → ${entry.returns}`,
        info: entry.doc,
        apply: `${entry.name}(${entry.params.map((param) => param.name).join(', ')})`,
      })),
    };
  }

  const apiHover = hoverTooltip((editor, position) => {
    const line = editor.state.doc.lineAt(position);
    const left = line.text.slice(0, position - line.from).match(/[\w.]+$/)?.[0] ?? '';
    const right = line.text.slice(position - line.from).match(/^[\w.]*/)?.[0] ?? '';
    const word = left + right;
    const entry = api.find((candidate) => candidate.name === word);
    if (!entry) return null;
    return {
      pos: position - left.length,
      end: position + right.length,
      above: true,
      create() {
        const dom = document.createElement('div');
        dom.className = 'cm-api-doc';
        const signature = document.createElement('code');
        signature.textContent = `${entry.name}(${entry.params.map((param) => `${param.name}: ${param.ty}`).join(', ')}) → ${entry.returns}`;
        const copy = document.createElement('p');
        copy.textContent = entry.doc;
        dom.append(signature, copy);
        return { dom };
      },
    };
  });

  function syncBreakpoints() {
    view?.dispatch({
      effects: setBreakpoints.of(
        breakpoints.filter((breakpoint) => breakpoint.source === path).map((breakpoint) => breakpoint.line),
      ),
    });
  }

  function applyInsert() {
    if (!view || !insertRequest || insertRequest.source !== path || insertRequest.id === handledInsert) return;
    handledInsert = insertRequest.id;
    const selection = view.state.selection.main;
    view.dispatch({
      changes: { from: selection.from, to: selection.to, insert: insertRequest.text },
      selection: { anchor: selection.from + insertRequest.text.length },
      scrollIntoView: true,
    });
    view.focus();
    onInsertHandled(insertRequest.id);
  }

  function applyReveal() {
    if (!view || !revealRequest || revealRequest.source !== path || revealRequest.id === handledReveal) return;
    handledReveal = revealRequest.id;
    const anchor = sourceOffset(view.state.doc.toString(), revealRequest.line, revealRequest.column);
    view.dispatch({
      selection: { anchor },
      effects: EditorView.scrollIntoView(anchor, { y: 'center' }),
    });
    view.focus();
    onRevealHandled(revealRequest.id);
  }

  /** Best-effort lexical scan (not a parser) for references to a disabled
   * prelude module's globals, so the editor can offer a quick-fix that
   * enables the module — not a completion, since the module isn't active.
   * Skips `.`/`:` member access (`foo.Vec2`) and `--` comment tails; false
   * positives on string literals containing the same text are acceptable. */
  function disabledModuleDiagnostics(): CmDiagnostic[] {
    if (!view) return [];
    const disabledGlobals = new Map<string, string>();
    for (const module of preludeModules) {
      if (module.enabled) continue;
      for (const global of module.globals) disabledGlobals.set(global, module.name);
    }
    if (disabledGlobals.size === 0) return [];

    const text = view.state.doc.toString();
    const items: CmDiagnostic[] = [];
    const wordPattern = /[A-Za-z_][A-Za-z0-9_]*/g;
    let match: RegExpExecArray | null;
    while ((match = wordPattern.exec(text))) {
      const word = match[0];
      const moduleName = disabledGlobals.get(word);
      if (!moduleName) continue;
      const start = match.index;
      const prevChar = start > 0 ? text[start - 1] : '';
      if (prevChar === '.' || prevChar === ':') continue;
      const lineStart = text.lastIndexOf('\n', start - 1) + 1;
      if (text.slice(lineStart, start).includes('--')) continue;
      items.push({
        from: start,
        to: start + word.length,
        severity: 'warning',
        message: `${word} not available — module '${moduleName}' not enabled`,
        actions: [{ name: `Enable '${moduleName}'`, apply: () => onEnableModule(moduleName) }],
      });
    }
    return items;
  }

  function syncDiagnostics() {
    if (!view) return;
    const items: CmDiagnostic[] = diagnostics
      .filter((item) => item.path === path && item.line)
      .map((item) => {
        const line = view!.state.doc.line(Math.min(item.line!, view!.state.doc.lines));
        return {
          from: line.from,
          to: line.to,
          severity: item.severity === 'error' ? 'error' : item.severity === 'info' ? 'info' : 'hint',
          message: `${item.title}: ${item.detail}`,
        };
      });
    view.dispatch(setDiagnostics(view.state, [...items, ...disabledModuleDiagnostics()]));
  }

  onMount(() => {
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: value,
        selection: { anchor: Math.max(0, Math.min(initialCursor, value.length)) },
        extensions: [
          lineNumbers(), highlightActiveLineGutter(), highlightSpecialChars(), history(),
          drawSelection(), dropCursor(), rectangularSelection(), bracketMatching(),
          syntaxHighlighting(luaHighlightStyle, { fallback: true }),
          StreamLanguage.define(lua), autocompletion({ override: [completions] }), apiHover,
          breakpointField,
          gutter({
            class: 'cm-breakpoint-gutter',
            markers: (editor) => editor.state.field(breakpointField),
            initialSpacer: () => marker,
            domEventHandlers: {
              mousedown(editor, block) {
                onToggleBreakpoint(path, editor.state.doc.lineAt(block.from).number);
                return true;
              },
            },
          }),
          highlightActiveLine(),
          keymap.of([...defaultKeymap, ...historyKeymap, ...completionKeymap, ...searchKeymap, ...lintKeymap, indentWithTab]),
          EditorView.updateListener.of((update) => {
            if (update.docChanged && !update.transactions.some((transaction) => transaction.annotation(externalDocument))) {
              onChange(update.state.doc.toString());
            }
            if (update.selectionSet || update.docChanged) onCursor(path, update.state.selection.main.head);
            if (update.docChanged) syncDiagnostics();
          }),
          EditorView.theme({
            '&': { height: '100%', backgroundColor: 'var(--color-void-900)', color: 'var(--color-ink)', fontSize: '13px' },
            '.cm-scroller': { fontFamily: 'var(--font-mono)', lineHeight: '1.72' },
            '.cm-content': { caretColor: 'var(--color-ember)', padding: '14px 0 40px' },
            '.cm-gutters': { backgroundColor: 'var(--color-void-800)', color: 'var(--color-ink-dim)', border: 'none' },
            '.cm-activeLineGutter, .cm-activeLine': { backgroundColor: 'var(--color-void-700)' },
            '.cm-selectionBackground, &.cm-focused .cm-selectionBackground': { backgroundColor: 'rgba(254,176,93,.28)' },
            '.cm-cursor': { borderLeftColor: 'var(--color-ember)' },
            '.cm-tooltip': { backgroundColor: 'var(--color-void-800)', border: '1px solid var(--color-void-600)', color: 'var(--color-ink)' },
          }),
        ],
      }),
    });
    syncBreakpoints();
    syncDiagnostics();
    applyInsert();
    applyReveal();
    return () => view?.destroy();
  });

  $effect(() => {
    value; path;
    if (view && view.state.doc.toString() !== value) {
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: value }, annotations: externalDocument.of(true) });
    }
  });
  $effect(() => { breakpoints; path; syncBreakpoints(); });
  $effect(() => { diagnostics; path; syncDiagnostics(); });
  $effect(() => { preludeModules; syncDiagnostics(); });
  $effect(() => { insertRequest; applyInsert(); });
  $effect(() => { revealRequest; applyReveal(); });
</script>

<div class="lua-editor" bind:this={host}></div>

<style>
  .lua-editor { flex: 1; width: 100%; height: 100%; min-width: 0; min-height: 0; overflow: hidden; }
  :global(.cm-breakpoint-gutter) { width: 14px; cursor: pointer; }
  :global(.cm-breakpoint-gutter .cm-gutterElement) { box-sizing: border-box; width: 14px; padding: 0; display: flex; align-items: center; justify-content: center; }
  :global(.cm-breakpoint-dot) { width: 8px; height: 8px; flex: none; display: block; margin: 0; border-radius: 50%; background: var(--color-ember); box-shadow: 0 0 6px rgba(254,176,93,.45); }
  :global(.cm-api-doc) { max-width: 370px; padding: 10px 12px; }
  :global(.cm-api-doc code) { color: #73daca; font-weight: 700; }
  :global(.cm-api-doc p) { margin: 7px 0 0; color: #aaaabc; line-height: 1.5; }
</style>
