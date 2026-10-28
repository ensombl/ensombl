import { HighlightStyle, syntaxHighlighting } from "@codemirror/language"
import { EditorView } from "@codemirror/view"
import { tags as t } from "@lezer/highlight"

// GitHub Dark Default, the palette the review view (Shiki) uses.
const c = {
  bg: "#0d1117",
  fg: "#e6edf3",
  muted: "#8b949e",
  gutter: "#6e7681",
  red: "#ff7b72",
  blue: "#79c0ff",
  lightBlue: "#a5d6ff",
  purple: "#d2a8ff",
  orange: "#ffa657",
  green: "#7ee787",
  panel: "#161b22",
  border: "#30363d",
}

const editor = EditorView.theme(
  {
    "&": { height: "100%", fontSize: "12.5px", color: c.fg, backgroundColor: c.bg },
    "&.cm-focused": { outline: "none" },
    ".cm-scroller": { fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace", lineHeight: "1.6" },
    ".cm-content": { caretColor: c.fg },
    ".cm-cursor, .cm-dropCursor": { borderLeftColor: c.fg },
    "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection": {
      backgroundColor: "#264f78",
    },
    ".cm-activeLine": { backgroundColor: "#6e76811a" },
    ".cm-gutters": { backgroundColor: c.bg, color: c.gutter, border: "none" },
    ".cm-activeLineGutter": { backgroundColor: "transparent", color: c.fg },
    ".cm-foldPlaceholder": { backgroundColor: "transparent", border: "none", color: c.muted },
    ".cm-matchingBracket, &.cm-focused .cm-matchingBracket": { backgroundColor: "#3fb95040", outline: "none" },
    ".cm-searchMatch": { backgroundColor: "#9e6a0340", outline: `1px solid ${c.border}` },
    ".cm-searchMatch.cm-searchMatch-selected": { backgroundColor: "#f2cc6080" },
    ".cm-selectionMatch": { backgroundColor: "#3fb95026" },
    ".cm-panels": { backgroundColor: c.panel, color: c.fg },
    ".cm-panels.cm-panels-top": { borderBottom: `1px solid ${c.border}` },
    ".cm-panels.cm-panels-bottom": { borderTop: `1px solid ${c.border}` },
    ".cm-tooltip": { backgroundColor: c.panel, border: `1px solid ${c.border}`, color: c.fg },
    ".cm-tooltip-autocomplete > ul > li[aria-selected]": { backgroundColor: "#1f6feb66", color: c.fg },
  },
  { dark: true },
)

const highlight = HighlightStyle.define([
  { tag: [t.comment, t.lineComment, t.blockComment, t.docComment], color: c.muted },
  { tag: [t.keyword, t.operatorKeyword, t.controlKeyword, t.definitionKeyword, t.moduleKeyword, t.modifier, t.operator], color: c.red },
  { tag: [t.string, t.special(t.string), t.regexp, t.url], color: c.lightBlue },
  { tag: [t.number, t.bool, t.null, t.atom, t.escape, t.constant(t.variableName), t.definition(t.variableName), t.attributeName], color: c.blue },
  { tag: [t.function(t.variableName), t.function(t.propertyName), t.function(t.definition(t.variableName)), t.meta, t.annotation], color: c.purple },
  { tag: [t.typeName, t.className, t.namespace, t.labelName], color: c.orange },
  { tag: t.tagName, color: c.green },
  { tag: t.heading, color: c.blue, fontWeight: "bold" },
  { tag: t.link, color: c.lightBlue, textDecoration: "underline" },
  { tag: t.emphasis, fontStyle: "italic" },
  { tag: t.strong, fontWeight: "bold" },
  { tag: t.invalid, color: "#ffa198" },
])

export const githubDark = [editor, syntaxHighlighting(highlight)]
