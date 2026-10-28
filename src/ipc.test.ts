import { afterEach, describe, expect, it } from "vitest";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { ipc } from "@/ipc";

describe("ipc", () => {
  afterEach(() => clearMocks());

  it("calls app_info", async () => {
    mockIPC((cmd) => (cmd === "app_info" ? { name: "CRM", version: "0.1.0" } : null));
    await expect(ipc.appInfo()).resolves.toEqual({ name: "CRM", version: "0.1.0" });
  });
});
