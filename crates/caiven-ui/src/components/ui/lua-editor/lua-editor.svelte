<script lang="ts">
  import { onMount } from 'svelte';
  import { Annotation, EditorState, Prec, RangeSet, StateEffect, StateField } from '@codemirror/state';
  import {
    Decoration, EditorView, GutterMarker, drawSelection, dropCursor, gutter, gutterLineClass, highlightActiveLine,
    highlightActiveLineGutter, highlightSpecialChars, hoverTooltip, keymap, lineNumbers,
    rectangularSelection,
  } from '@codemirror/view';
  import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
  import { acceptCompletion, autocompletion, completionKeymap, startCompletion, type Completion, type CompletionContext } from '@codemirror/autocomplete';
  import { bracketMatching, HighlightStyle, syntaxHighlighting, StreamLanguage } from '@codemirror/language';
  import { lintKeymap, setDiagnostics, type Diagnostic as CmDiagnostic } from '@codemirror/lint';
  import { searchKeymap } from '@codemirror/search';
  import { lua } from '@codemirror/legacy-modes/mode/lua';
  import { tags as t } from '@lezer/highlight';
  import {
    availableApi, builtinAliasEntries, declaresLocal, localName, moduleKey, projectEntries, requireNameAt, scanModule,
    sourceOffset, watchPathAt, type ApiEntry, type PreludeModule, type ProjectModule,
  } from './lua-modules';

  type Diagnostic = { severity: string; title: string; detail: string; path: string; line: number | null };
  type Breakpoint = { source: string; line: number };
  type EditorInsertRequest = { id: number; source: string; text: string };
  /** `length` selects that many characters from the revealed position. */
  type EditorRevealRequest = { id: number; source: string; line: number; column: number; length?: number };

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
    { tag: t.standard(t.variableName), color: 'var(--color-ember-bright)' },
    { tag: t.variableName, color: 'var(--color-ink)' },
    { tag: t.operator, color: 'var(--color-ink-dim)' },
    { tag: [t.bracket, t.punctuation], color: 'var(--color-ink-dim)' },
  ]);

  interface Props {
    value: string;
    path?: string;
    initialCursor?: number;
    api: ApiEntry[];
    preludeModules: PreludeModule[];
    diagnostics?: Diagnostic[];
    breakpoints?: Breakpoint[];
    /** Line of this source the paused frame stopped on. */
    pausedLine?: number | null;
    /** Reads a value for the hover while paused; null otherwise. */
    onPeek?: ((expression: string) => Promise<string | null>) | null;
    insertRequest?: EditorInsertRequest | null;
    revealRequest?: EditorRevealRequest | null;
    onInsertHandled?: (id: number) => void;
    onRevealHandled?: (id: number) => void;
    onChange: (value: string) => void;
    onCursor?: (source: string, offset: number) => void;
    /** Without it there is no breakpoint gutter. */
    onToggleBreakpoint?: (source: string, line: number) => void;
    /** Bound to Mod-Enter. */
    onRun?: () => void;
    /** Every cart source, for `require` names and project symbols. */
    projectModules?: ProjectModule[];
    label?: string;
  }

  let {
    value, path = 'main.lua', initialCursor = 0, api, preludeModules, diagnostics = [], breakpoints = [], pausedLine = null,
    onPeek = null, insertRequest = null, revealRequest = null, onInsertHandled, onRevealHandled, onChange, onCursor,
    onToggleBreakpoint, onRun, projectModules = [], label = 'Lua source',
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

  class PausedGutterMarker extends GutterMarker {
    elementClass = 'cm-paused-gutter';
  }
  const pausedGutter = new PausedGutterMarker();
  const pausedLineDecoration = Decoration.line({ class: 'cm-paused-line' });
  const setPausedLine = StateEffect.define<number | null>();
  // Start of the line the frame stopped on; edits carry it along.
  const pausedField = StateField.define<number | null>({
    create: () => null,
    update(position, transaction) {
      for (const effect of transaction.effects) if (effect.is(setPausedLine)) return effect.value;
      return position === null ? null : transaction.changes.mapPos(position);
    },
    provide: (field) => [
      EditorView.decorations.compute([field], (state) => {
        const position = state.field(field);
        return position === null ? Decoration.none : Decoration.set([pausedLineDecoration.range(state.doc.lineAt(position).from)]);
      }),
      gutterLineClass.compute([field], (state) => {
        const position = state.field(field);
        return position === null ? RangeSet.empty : RangeSet.of([pausedGutter.range(state.doc.lineAt(position).from)]);
      }),
    ],
  });

  const requirable = () => projectModules.filter((module) => !module.entry);
  const signature = (entry: ApiEntry, typed = false) =>
    `(${entry.params.map((param) => (typed && param.ty ? `${param.name}: ${param.ty}` : param.name)).join(', ')})${entry.returns ? ` → ${entry.returns}` : ''}`;

  type Apply = string | ((editor: EditorView, completion: Completion, from: number, to: number) => void);

  /** Module names for a `require`; `apply` builds the insertion from the key and its conventional local. */
  function moduleOptions(apply: (key: string, local: string) => Apply) {
    const own = requirable().map((module) => ({ label: module.key, type: 'namespace', detail: 'project module', apply: apply(module.key, localName(module.key)) }));
    const builtin = preludeModules
      .filter((module) => !own.some((option) => option.label === module.name))
      .map((module) => ({
        label: module.name, type: 'namespace', detail: 'built-in', info: `local ${module.export} = require "${module.name}"`,
        apply: apply(module.name, module.export),
      }));
    return [...own, ...builtin];
  }

  // A `require` alone on its line becomes the whole conventional line,
  // swallowing any closing quote/paren already typed after the cursor.
  const localLine = (start: number, key: string, local: string): Apply => (editor, _completion, _from, to) => {
    let end = to;
    if (/["']/.test(editor.state.sliceDoc(end, end + 1))) end += 1;
    if (editor.state.sliceDoc(end, end + 1) === ')') end += 1;
    const insert = `local ${local} = require "${key}"`;
    editor.dispatch({ changes: { from: start, to: end, insert }, selection: { anchor: start + insert.length } });
  };

  // `require ""` with the cursor between the quotes, then the module list.
  const applyRequire = (editor: EditorView, _completion: Completion, from: number, to: number) => {
    editor.dispatch({ changes: { from, to, insert: 'require ""' }, selection: { anchor: from + 9 } });
    startCompletion(editor);
  };

  /** Everything the file can call: console API, built-ins under their aliases, project symbols. */
  function visibleEntries(text: string): ApiEntry[] {
    const shadowed = requirable().map((module) => module.key);
    return [...availableApi(api, preludeModules, text, shadowed), ...builtinAliasEntries(api, preludeModules, text, shadowed), ...projectEntries(projectModules, text, moduleKey(path))];
  }

  function completions(context: CompletionContext) {
    const call = context.matchBefore(/require\s*(\(\s*)?(["'][\w./]*)?/);
    if (call && /^require\s*(\(\s*)?(["']|\s|\()/.test(call.text)) {
      const lineStart = context.state.doc.lineAt(call.from).from;
      const bare = /^\s*$/.test(context.state.sliceDoc(lineStart, call.from));
      const quote = call.text.search(/["']/);
      const paren = call.text.includes('(');
      const apply = (key: string, local: string): Apply => {
        if (bare) return localLine(call.from, key, local);
        if (quote >= 0) return key;
        return paren ? `"${key}")` : `"${key}"`;
      };
      const from = quote >= 0 ? call.from + quote + 1 : context.pos;
      return { from, options: moduleOptions(apply), validFor: /^[\w./]*$/ };
    }
    const word = context.matchBefore(/[\w.]*/);
    if (!word || (!context.explicit && word.from === word.to)) return null;
    return {
      from: word.from,
      options: visibleEntries(context.state.doc.toString()).map((entry) => {
        const value = entry.category === 'Project value';
        return {
          label: entry.name,
          type: value ? 'variable' : 'function',
          detail: value ? entry.category : signature(entry),
          info: entry.doc,
          apply: entry.name === 'require' ? applyRequire
            : value ? entry.name : `${entry.name}(${entry.params.map((param) => param.name).join(', ')})`,
        };
      }),
    };
  }

  function requireHover(name: string): { title: string; body: string } | null {
    const own = requirable().find((module) => module.key === name);
    if (own) {
      const scan = scanModule(own.key, own.text);
      const names = [...scan.exports.map((entry) => `.${entry.name}`), ...scan.globals.map((entry) => entry.name)];
      return { title: `${name.replace(/\./g, '/')}.lua`, body: names.length ? `Project module. Provides ${names.join(', ')}.` : 'Project module.' };
    }
    const builtin = preludeModules.find((module) => module.name === name);
    if (!builtin) return null;
    const members = api.filter((entry) => new RegExp(`^${builtin.export}[.:]`).test(entry.name)).map((entry) => entry.name.slice(builtin.export.length));
    return { title: `local ${builtin.export} = require "${name}"`, body: `Built-in module. Provides ${members.join(', ')}.` };
  }

  const apiHover = hoverTooltip((editor, position) => {
    const line = editor.state.doc.lineAt(position);
    const card = (pos: number, end: number, title: string, body: string) => ({
      pos,
      end,
      above: true,
      create() {
        const dom = document.createElement('div');
        dom.className = 'cm-api-doc';
        const heading = document.createElement('code');
        heading.textContent = title;
        const copy = document.createElement('p');
        copy.textContent = body;
        dom.append(heading, copy);
        return { dom };
      },
    });
    const required = requireNameAt(line.text, position - line.from);
    if (required) {
      const info = requireHover(required.name);
      return info ? card(line.from + required.from, line.from + required.to, info.title, info.body) : null;
    }
    const left = line.text.slice(0, position - line.from).match(/[\w.]+$/)?.[0] ?? '';
    const right = line.text.slice(position - line.from).match(/^[\w.]*/)?.[0] ?? '';
    const word = left + right;
    const entry = visibleEntries(editor.state.doc.toString()).find((candidate) => candidate.name === word);
    if (!entry) return null;
    const title = entry.category === 'Project value' ? entry.name : `${entry.name}${signature(entry, true)}`;
    return card(position - left.length, position + right.length, title, entry.doc);
  });

  const valueHover = hoverTooltip(async (editor, position, side) => {
    if (!onPeek) return null;
    const line = editor.state.doc.lineAt(position);
    const column = position - line.from - (side < 0 ? 1 : 0);
    if (line.text.slice(0, column).includes('--')) return null;
    const target = watchPathAt(line.text, column);
    if (!target || api.some((entry) => entry.name === target.path)) return null;
    const value = await onPeek(target.path);
    if (value === null) return null;
    return {
      pos: line.from + target.from,
      end: line.from + target.to,
      above: true,
      create() {
        const dom = document.createElement('div');
        dom.className = 'cm-value-peek';
        const name = document.createElement('code');
        name.textContent = target.path;
        dom.append(name, ` = ${value}`);
        return { dom };
      },
    };
  });

  let breakpointTimer: ReturnType<typeof setTimeout> | undefined;

  // Edits move gutter markers; push the moved lines back so the backend does not stop on stale ones.
  function reconcileBreakpoints() {
    if (!view) return;
    const doc = view.state.doc;
    const markerLines = new Set<number>();
    view.state.field(breakpointField).between(0, doc.length, (from) => {
      markerLines.add(doc.lineAt(from).number);
    });
    const known = new Set(breakpoints.filter((breakpoint) => breakpoint.source === path).map((breakpoint) => breakpoint.line));
    for (const line of known) if (!markerLines.has(line)) onToggleBreakpoint?.(path, line);
    for (const line of markerLines) if (!known.has(line)) onToggleBreakpoint?.(path, line);
  }

  function syncBreakpoints() {
    view?.dispatch({
      effects: setBreakpoints.of(
        breakpoints.filter((breakpoint) => breakpoint.source === path).map((breakpoint) => breakpoint.line),
      ),
    });
  }

  function syncPausedLine() {
    if (!view) return;
    const doc = view.state.doc;
    view.dispatch({ effects: setPausedLine.of(pausedLine && pausedLine <= doc.lines ? doc.line(pausedLine).from : null) });
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
    onInsertHandled?.(insertRequest.id);
  }

  function applyReveal() {
    if (!view || !revealRequest || revealRequest.source !== path || revealRequest.id === handledReveal) return;
    handledReveal = revealRequest.id;
    const anchor = sourceOffset(view.state.doc.toString(), revealRequest.line, revealRequest.column);
    view.dispatch({
      selection: { anchor, head: Math.min(anchor + (revealRequest.length ?? 0), view.state.doc.length) },
      effects: EditorView.scrollIntoView(anchor, { y: 'center' }),
    });
    view.focus();
    onRevealHandled?.(revealRequest.id);
  }

  /** Best-effort lexical scan (not a parser) for a module's conventional
   * local (`Camera`, `tween`) used in a file that never declares it, with a
   * quick-fix adding the `local … = require` line. Skips `.`/`:` member
   * access and `--` comment tails; false positives on strings are fine. */
  function missingRequireDiagnostics(): CmDiagnostic[] {
    if (!view) return [];
    const text = view.state.doc.toString();
    const disabledGlobals = new Map<string, string>();
    for (const module of preludeModules) {
      if (!declaresLocal(text, module.export)) disabledGlobals.set(module.export, module.name);
    }
    if (disabledGlobals.size === 0) return [];

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
        message: `${word} comes from module '${moduleName}' — add local ${word} = require "${moduleName}"`,
        actions: [{
          name: `Add local ${word} = require "${moduleName}"`,
          apply: (editor) => editor.dispatch({ changes: { from: 0, insert: `local ${word} = require "${moduleName}"\n` } }),
        }],
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
    view.dispatch(setDiagnostics(view.state, [...items, ...missingRequireDiagnostics()]));
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
          StreamLanguage.define(lua), autocompletion({ override: [completions] }), apiHover, valueHover,
          breakpointField, pausedField,
          EditorView.contentAttributes.of({ 'aria-label': label }),
          onRun ? Prec.highest(keymap.of([{ key: 'Mod-Enter', run: () => { onRun?.(); return true; } }])) : [],
          !onToggleBreakpoint ? [] : gutter({
            class: 'cm-breakpoint-gutter',
            markers: (editor) => editor.state.field(breakpointField),
            initialSpacer: () => marker,
            domEventHandlers: {
              mousedown(editor, block) {
                onToggleBreakpoint?.(path, editor.state.doc.lineAt(block.from).number);
                return true;
              },
            },
          }),
          highlightActiveLine(),
          keymap.of([...defaultKeymap, ...historyKeymap, ...completionKeymap, ...searchKeymap, ...lintKeymap, { key: 'Tab', run: acceptCompletion }, indentWithTab]),
          EditorView.updateListener.of((update) => {
            if (update.docChanged && !update.transactions.some((transaction) => transaction.annotation(externalDocument))) {
              onChange(update.state.doc.toString());
            }
            if (update.selectionSet || update.docChanged) onCursor?.(path, update.state.selection.main.head);
            if (update.docChanged) syncDiagnostics();
            if (update.docChanged && !update.transactions.some((transaction) => transaction.annotation(externalDocument))) {
              clearTimeout(breakpointTimer);
              breakpointTimer = setTimeout(reconcileBreakpoints, 400);
            }
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
    syncPausedLine();
    syncDiagnostics();
    applyInsert();
    applyReveal();
    return () => {
      clearTimeout(breakpointTimer);
      view?.destroy();
    };
  });

  $effect(() => {
    value; path;
    if (view && view.state.doc.toString() !== value) {
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: value }, annotations: externalDocument.of(true) });
    }
  });
  $effect(() => { breakpoints; path; syncBreakpoints(); });
  $effect(() => { pausedLine; syncPausedLine(); });
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
  :global(.cm-paused-line) { background: rgba(245,197,66,.13); box-shadow: inset 2px 0 0 #f5c542; }
  :global(.cm-gutterElement.cm-paused-gutter) { color: #f5c542; font-weight: 700; }
  :global(.cm-value-peek) { max-width: 370px; padding: 6px 10px; font-family: var(--font-mono); font-size: 12px; overflow-wrap: anywhere; }
  :global(.cm-value-peek code) { color: #73daca; font-weight: 700; }
  :global(.cm-api-doc) { max-width: 370px; padding: 10px 12px; }
  :global(.cm-api-doc code) { color: #73daca; font-weight: 700; }
  :global(.cm-api-doc p) { margin: 7px 0 0; color: #aaaabc; line-height: 1.5; }
</style>
