import { describe, expect, it } from "vitest";
import { resolveApiBaseUrl, resolveWsBaseUrl } from "./runtimeEndpoints";

describe("runtime endpoint resolution", () => {
  it("uses explicit API URL when configured", () => {
    expect(resolveApiBaseUrl("https://api.example.test", "tauri:", "localhost")).toBe(
      "https://api.example.test",
    );
  });

  it("uses localhost HTTP endpoints for Tauri custom protocol windows", () => {
    expect(resolveApiBaseUrl(undefined, "tauri:", "localhost")).toBe("http://localhost:3000");
    expect(resolveWsBaseUrl(undefined, "tauri:", "localhost")).toBe("ws://localhost:3000");
  });

  it("keeps browser protocol semantics for web development", () => {
    expect(resolveApiBaseUrl(undefined, "http:", "127.0.0.1")).toBe("http://127.0.0.1:3000");
    expect(resolveApiBaseUrl(undefined, "https:", "chat.example.test")).toBe(
      "https://chat.example.test:3000",
    );
    expect(resolveWsBaseUrl(undefined, "https:", "chat.example.test")).toBe(
      "wss://chat.example.test:3000",
    );
  });
});
