import { describe, expect, it } from "vitest";
import { formatFileSize, parseFileMessage } from "./fileMessage";

describe("parseFileMessage", () => {
  it("parses file messages with upload metadata", () => {
    expect(
      parseFileMessage("file::/uploads/attachments/report.pdf::report.pdf::application/pdf::1536"),
    ).toEqual({
      url: "/uploads/attachments/report.pdf",
      filename: "report.pdf",
      mime: "application/pdf",
      size: 1536,
    });
  });

  it("returns null for regular chat content", () => {
    expect(parseFileMessage("hello team")).toBeNull();
  });
});

describe("formatFileSize", () => {
  it("formats bytes, kilobytes and megabytes", () => {
    expect(formatFileSize(512)).toBe("512 B");
    expect(formatFileSize(1536)).toBe("1.5 KB");
    expect(formatFileSize(2 * 1024 * 1024)).toBe("2.0 MB");
  });
});
