import { describe, expect, it } from "vitest";
import { filterMessagesBySearchQuery } from "./messageSearch";
import type { ChatMessage } from "./api";

const messages: ChatMessage[] = [
  { id: "1", author_id: "u1", content: "The deployment checklist is ready" },
  { id: "2", author_id: "u2", content: "gif::https://media.giphy.com/media/example/giphy.gif" },
  { id: "3", author_id: "u1", content: "Remember to pin the investor demo notes" },
  {
    id: "4",
    author_id: "u2",
    content: "file::/uploads/attachments/demo.pdf::demo.pdf::application/pdf::2048",
  },
];

describe("filterMessagesBySearchQuery", () => {
  it("returns all messages when query is blank", () => {
    expect(filterMessagesBySearchQuery(messages, "   ")).toEqual(messages);
  });

  it("matches message content case-insensitively inside the current conversation", () => {
    expect(filterMessagesBySearchQuery(messages, "INVESTOR")).toEqual([messages[2]]);
  });

  it("matches gif messages by their stored content", () => {
    expect(filterMessagesBySearchQuery(messages, "giphy")).toEqual([messages[1]]);
  });

  it("matches file messages by filename", () => {
    expect(filterMessagesBySearchQuery(messages, "demo.pdf")).toEqual([messages[3]]);
  });
});
