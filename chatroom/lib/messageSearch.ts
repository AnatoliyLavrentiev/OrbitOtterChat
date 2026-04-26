import type { ChatMessage } from "./api";
import { parseFileMessage } from "./fileMessage";

export const filterMessagesBySearchQuery = (
  messages: ChatMessage[],
  query: string,
): ChatMessage[] => {
  const normalized = query.trim().toLowerCase();
  if (!normalized) return messages;
  return messages.filter((message) => {
    const file = parseFileMessage(message.content);
    const searchable = file
      ? `${file.filename} ${file.mime} ${message.content}`
      : message.content;
    return searchable.toLowerCase().includes(normalized);
  });
};
