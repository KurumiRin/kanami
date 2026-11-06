// Config source editor: CodeMirror 6 with TOML highlighting, JSON Schema completion, and validation errors and test results.
import { t } from "./i18n.svelte.js";
import { EditorView, keymap, lineNumbers, highlightActiveLine, drawSelection, gutter, GutterMarker } from "@codemirror/view";
import { EditorState, StateEffect, StateField, RangeSet } from "@codemirror/state";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { StreamLanguage, syntaxHighlighting, HighlightStyle, bracketMatching } from "@codemirror/language";
import { toml } from "@codemirror/legacy-modes/mode/toml";
import { autocompletion, completionKeymap } from "@codemirror/autocomplete";
import { setDiagnostics } from "@codemirror/lint";
import { tags } from "@lezer/highlight";

const highlight = HighlightStyle.define([
  { tag: tags.comment, color: "var(--ink-3)" },
  { tag: [tags.propertyName, tags.variableName], color: "var(--accent-ink)" },
  { tag: tags.string, color: "var(--ok)" },
  { tag: [tags.number, tags.bool, tags.atom], color: "var(--warn)" },
  { tag: [tags.heading, tags.typeName, tags.bracket], fontWeight: "700" },
]);

const theme = EditorView.theme({
  "&": { height: "100%", fontSize: "12.5px", backgroundColor: "transparent", color: "var(--ink)" },
  ".cm-scroller": { fontFamily: "var(--mono)", lineHeight: "1.75" },
  ".cm-content": { caretColor: "var(--accent)", padding: "10px 0" },
  ".cm-cursor": { borderLeftColor: "var(--accent)", borderLeftWidth: "1.5px" },
  ".cm-gutters": { backgroundColor: "transparent", border: "none", color: "var(--ink-3)" },
  ".cm-activeLine": { backgroundColor: "rgba(127,127,127,.06)" },
  ".cm-activeLineGutter": { backgroundColor: "transparent", color: "var(--ink-2)" },
  "&.cm-focused .cm-selectionBackground, .cm-selectionBackground": { backgroundColor: "var(--accent-soft) !important" },
  ".cm-tooltip": { border: "1px solid var(--line)", backgroundColor: "var(--win-bg)", borderRadius: "8px", boxShadow: "0 8px 24px rgba(0,0,0,.18)" },
  ".cm-tooltip-autocomplete > ul > li": { padding: "3px 8px", fontFamily: "var(--mono)" },
  ".cm-tooltip-autocomplete > ul > li[aria-selected]": { backgroundColor: "var(--accent-soft)", color: "var(--ink)" },
  ".cm-completionDetail": { fontFamily: "var(--font)", color: "var(--ink-3)", fontStyle: "normal", marginLeft: "1em" },
  ".cm-diagnostic-error": { borderLeftColor: "var(--err)" },
  ".cm-diagnostic-warning": { borderLeftColor: "var(--warn)" },
  ".cm-lintRange-error": { backgroundImage: "none", textDecoration: "wavy underline var(--err)", textUnderlineOffset: "3px" },
  ".cm-lintRange-warning": { backgroundImage: "none", textDecoration: "wavy underline var(--warn)", textUnderlineOffset: "3px" },
  ".cm-test-gutter": { width: "18px" },
  ".cm-test-pass": { color: "var(--ok)", fontWeight: "700" },
  ".cm-test-fail": { color: "var(--err)", fontWeight: "700" },
});

// ---- Gutter marks for test results: the [[test]] line shows ✓ or ✗ ----

class TestMark extends GutterMarker {
  constructor(passed) {
    super();
    this.passed = passed;
  }
  eq(other) {
    return other.passed === this.passed;
  }
  toDOM() {
    const span = document.createElement("span");
    span.className = this.passed ? "cm-test-pass" : "cm-test-fail";
    span.textContent = this.passed ? "✓" : "✗";
    span.title = this.passed ? t("Test passed") : t("Test failed");
    return span;
  }
}

const setTests = StateEffect.define();
const testMarks = StateField.define({
  create: () => RangeSet.empty,
  update(marks, tr) {
    marks = marks.map(tr.changes);
    for (const effect of tr.effects) if (effect.is(setTests)) marks = effect.value;
    return marks;
  },
});

// ---- Completion: key names, field names and table names all come from the JSON Schema ----

function vocabulary(schemaText) {
  const schema = JSON.parse(schemaText);
  const keys = schema.definitions?.key?.enum ?? [];
  const fields = new Set();
  const walk = (node) => {
    if (!node || typeof node !== "object") return;
    if (node.properties) for (const [name, child] of Object.entries(node.properties)) {
      fields.add(name);
      walk(child);
    }
    for (const value of Object.values(node)) if (typeof value === "object") walk(value);
  };
  walk(schema);
  const tables = Object.keys(schema.properties ?? {});
  const modifiers = ["cmd", "ctrl", "opt", "alt", "shift", "fn", "hyper", "meh", "none"];
  return { keys, fields: [...fields], tables, modifiers };
}

function completions(words) {
  return (context) => {
    // Inside a string: key names and modifiers; combos like "cmd+k" work
    const inString = context.matchBefore(/"[\w+]*/);
    if (inString) {
      const word = context.matchBefore(/[\w]*/);
      const options = [
        ...words.keys.map((k) => ({ label: k, type: "constant" })),
        ...words.modifiers.map((m) => ({ label: m, type: "keyword", detail: t("modifier") })),
      ];
      return { from: word.from, options, validFor: /^\w*$/ };
    }
    // Inside brackets: table names
    const table = context.matchBefore(/^\s*\[\[?[\w.]*/);
    if (table) {
      const word = context.matchBefore(/[\w.]*/);
      return { from: word.from, options: words.tables.map((t) => ({ label: t, type: "class" })) };
    }
    // Start of line: field names
    const field = context.matchBefore(/^\s*[\w.]*/);
    if (field && (field.text.trim() || context.explicit)) {
      const word = context.matchBefore(/[\w.]*/);
      return { from: word.from, options: words.fields.map((f) => ({ label: f, type: "property" })) };
    }
    return null;
  };
}

export function createEditor(parent, { doc, schema, onChange, onSave }) {
  const words = vocabulary(schema);
  const view = new EditorView({
    parent,
    state: EditorState.create({
      doc,
      extensions: [
        lineNumbers(),
        gutter({ class: "cm-test-gutter", markers: (v) => v.state.field(testMarks) }),
        testMarks,
        history(),
        drawSelection(),
        highlightActiveLine(),
        bracketMatching(),
        EditorView.lineWrapping,
        StreamLanguage.define(toml),
        syntaxHighlighting(highlight),
        autocompletion({ override: [completions(words)], icons: false }),
        keymap.of([
          { key: "Mod-s", preventDefault: true, run: () => (onSave(), true) },
          ...completionKeymap,
          ...defaultKeymap,
          ...historyKeymap,
          indentWithTab,
        ]),
        theme,
        EditorView.updateListener.of((u) => {
          if (u.docChanged) onChange(u.state.doc.toString());
        }),
      ],
    }),
  });

  // Line and column (1-based) → document position
  const pos = (line, column = 1) => {
    const l = view.state.doc.line(Math.min(Math.max(line, 1), view.state.doc.lines));
    return Math.min(l.from + column - 1, l.to);
  };

  return {
    view,
    setDoc(text) {
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text } });
    },
    // check: the result of check_config on the Rust side
    showCheck(check) {
      const diagnostics = [];
      if (check.problem) {
        const from = pos(check.problem.line, check.problem.column);
        const line = view.state.doc.lineAt(from);
        const to = Math.max(from + 1, Math.min(line.to, view.state.doc.length));
        diagnostics.push({ from, to: from === to ? to : to, severity: "error", message: check.problem.message });
      }
      const marks = [];
      for (const test of check.tests ?? []) {
        const at = view.state.doc.line(Math.min(test.line, view.state.doc.lines));
        marks.push(new TestMark(test.passed).range(at.from));
        if (!test.passed) {
          diagnostics.push({
            from: at.from,
            to: at.to,
            severity: "warning",
            message: t("Test \"{name}\" failed\nExpected: {expected}\nActual: {actual}", { name: test.name, expected: test.expected, actual: test.actual }),
          });
        }
      }
      view.dispatch(setDiagnostics(view.state, diagnostics));
      view.dispatch({ effects: setTests.of(RangeSet.of(marks, true)) });
    },
    jump(line) {
      const at = pos(line);
      view.dispatch({ selection: { anchor: at }, scrollIntoView: true });
      view.focus();
    },
    destroy: () => view.destroy(),
  };
}
