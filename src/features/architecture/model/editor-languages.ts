import type { Extension } from "@codemirror/state"
import { css } from "@codemirror/lang-css"
import { html } from "@codemirror/lang-html"
import { javascript } from "@codemirror/lang-javascript"
import { json } from "@codemirror/lang-json"
import { markdown } from "@codemirror/lang-markdown"
import { python } from "@codemirror/lang-python"
import { rust } from "@codemirror/lang-rust"
import { yaml } from "@codemirror/lang-yaml"

/** Editor language for a file path; unknown types are plain text. */
export function languageFor(path: string): Extension {
  const ext = path.slice(path.lastIndexOf(".") + 1).toLowerCase()
  switch (ext) {
    case "ts":
    case "mts":
    case "cts":
      return javascript({ typescript: true })
    case "tsx":
      return javascript({ typescript: true, jsx: true })
    case "js":
    case "mjs":
    case "cjs":
      return javascript()
    case "jsx":
      return javascript({ jsx: true })
    case "json":
      return json()
    case "css":
    case "scss":
      return css()
    case "html":
    case "vue":
    case "svelte":
      return html()
    case "md":
    case "mdx":
      return markdown()
    case "rs":
      return rust()
    case "py":
      return python()
    case "yml":
    case "yaml":
      return yaml()
    default:
      return []
  }
}
