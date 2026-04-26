export type FileMessage = {
  url: string;
  filename: string;
  mime: string;
  size: number;
};

export const parseFileMessage = (content: string): FileMessage | null => {
  if (!content.startsWith("file::")) return null;

  const [, url, filename, mime, size] = content.split("::");
  const parsedSize = Number(size);

  if (!url || !filename || !mime || !Number.isFinite(parsedSize) || parsedSize < 0) {
    return null;
  }

  return {
    url,
    filename,
    mime,
    size: parsedSize,
  };
};

export const formatFileSize = (size: number): string => {
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
  return `${(size / (1024 * 1024)).toFixed(1)} MB`;
};
