import { describe, expect, it } from "vitest";
import nextConfig from "../next.config";
import tauriConfig from "../src-tauri/tauri.conf.json";

describe("desktop release frontend assets", () => {
  it("builds and packages the static Next export for Tauri", () => {
    expect(nextConfig.output).toBe("export");
    expect(tauriConfig.build.frontendDist).toBe("../out");
  });
});
