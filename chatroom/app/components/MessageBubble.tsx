interface Props {
  content: string;
  author: string;
  createdAt: string;
}

export default function MessageBubble({ content, author, createdAt }: Props) {
  const isGif = content.startsWith("gif::");
  const gifUrl = isGif ? content.slice("gif::".length) : null;

  return (
    <div className="flex flex-col gap-0.5 rounded-lg bg-zinc-800 px-3 py-2">
      <div className="flex items-baseline gap-2">
        <span className="text-sm font-semibold text-zinc-200">{author}</span>
        <span className="text-xs text-zinc-500">{new Date(createdAt).toLocaleTimeString()}</span>
      </div>
      {isGif ? (
        <img
          src={gifUrl!}
          alt="GIF"
          className="mt-1 max-w-[200px] rounded-lg"
          loading="lazy"
        />
      ) : (
        <p className="text-sm text-zinc-100">{content}</p>
      )}
    </div>
  );
}