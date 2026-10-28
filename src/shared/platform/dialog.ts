import { open } from "@tauri-apps/plugin-dialog"

/** Native folder picker. Resolves to `null` when cancelled. */
export async function pickFolder(title: string): Promise<string | null> {
  const picked = await open({ directory: true, multiple: false, title })
  return typeof picked === "string" ? picked : null
}
