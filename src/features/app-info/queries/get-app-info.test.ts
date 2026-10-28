import { afterEach, describe, expect, it } from "vitest"
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks"

import { getAppInfo } from "./get-app-info"

describe("getAppInfo", () => {
  afterEach(() => clearMocks())

  it("dispatches the get_app_info query", async () => {
    mockIPC((cmd) => (cmd === "get_app_info" ? { name: "CRM", version: "0.1.0" } : null))
    await expect(getAppInfo()).resolves.toEqual({ name: "CRM", version: "0.1.0" })
  })
})
