import { invoke } from "@tauri-apps/api/core";
import type { AppInfo } from "@/bindings/AppInfo";

/** Typed wrappers around Tauri commands. Keep in sync with `src-tauri/src/lib.rs`. */
export const ipc = {
  appInfo: () => invoke<AppInfo>("app_info"),
};
