"use client";
import { useState, useEffect, useRef } from "react";

const GIPHY_KEY = process.env.NEXT_PUBLIC_GIPHY_API_KEY!;
const GIPHY_BASE = "https://api.giphy.com/v1/gifs";

interface GifItem {
  id: string;
  title: string;
  previewUrl?: string;
  originalUrl?: string;
  username: string;
  source: string;
}

interface GiphyImageVariant {
  url: string;
}

interface GiphyImages {
  fixed_height_small?: GiphyImageVariant;
  downsized?: GiphyImageVariant;
  original?: GiphyImageVariant;
  fixed_height?: GiphyImageVariant;
}

interface GiphyGif {
  id: string;
  title?: string;
  images?: GiphyImages;
  username?: string;
  url: string;
}

interface Props {
  onSelect: (gifUrl: string) => void;
  onClose: () => void;
}

function mapGifs(data: GiphyGif[]): GifItem[] {
  return data.flatMap((g) => {
    const previewUrl = g.images?.fixed_height_small?.url ?? g.images?.downsized?.url;
    const originalUrl = g.images?.original?.url ?? g.images?.fixed_height?.url;
    if (!previewUrl || !originalUrl) return [];
    return [{
      id: g.id,
      title: g.title ?? "GIF",
      previewUrl,
      originalUrl,
      username: g.username || "giphy",
      source: g.url ?? `https://giphy.com/gifs/${g.id}`,
    }];
  });
}

export default function GifPicker({ onSelect, onClose }: Props) {
  const [query, setQuery] = useState("");
  const [gifs, setGifs] = useState<GifItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hoveredGif, setHoveredGif] = useState<GifItem | null>(null);
  const ref = useRef<HTMLDivElement>(null);


  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [onClose]);


  useEffect(() => {
    fetchTrending();
  }, []);


  useEffect(() => {
    if (query.length < 2) {
      fetchTrending();
      return;
    }
    const timer = setTimeout(() => fetchSearch(query), 400);
    return () => clearTimeout(timer);
  }, [query]);

  const fetchTrending = async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(
        `${GIPHY_BASE}/trending?api_key=${GIPHY_KEY}&limit=12&rating=g`
      );
      if (!res.ok) throw new Error(`Erreur ${res.status}`);
      const data = (await res.json()) as { data?: GiphyGif[] };
      setGifs(mapGifs(data.data ?? []));
    } catch {
      setError("Impossible de charger les GIFs trending.");
    } finally {
      setLoading(false);
    }
  };

  const fetchSearch = async (q: string) => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(
        `${GIPHY_BASE}/search?api_key=${GIPHY_KEY}&q=${encodeURIComponent(q)}&limit=12&rating=g`
      );
      if (!res.ok) throw new Error(`Erreur ${res.status}`);
      const data = (await res.json()) as { data?: GiphyGif[] };
      setGifs(mapGifs(data.data ?? []));
    } catch {
      setError("Impossible de charger les GIFs.");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div
      ref={ref}
      className="absolute bottom-14 left-0 z-50 w-80 rounded-2xl border border-zinc-700 bg-zinc-900 p-3 shadow-2xl"
    >
      
      <div className="mb-2 flex items-center justify-between">
        <span className="text-xs font-semibold text-zinc-400">
          {query.length >= 2 ? `Résultats pour "${query}"` : "🔥 Trending"}
        </span>
        
        <img
          src="https://developers.giphy.com/branch/master/static/header-logo-8974b8ae658f704a5b48a2d039b8ad6.gif"
          alt="Powered by GIPHY"
          className="h-4"
          title="Powered by GIPHY"
        />
      </div>

      
      <input
        type="text"
        placeholder="🔍 Rechercher un GIF..."
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        className="mb-2 w-full rounded-lg bg-zinc-800 px-3 py-2 text-sm text-zinc-100 placeholder-zinc-500 outline-none focus:ring-1 focus:ring-zinc-500"
        autoFocus
      />

      
      <div className="grid max-h-60 grid-cols-3 gap-1 overflow-y-auto">
        {loading ? (
          <p className="col-span-3 py-6 text-center text-xs text-zinc-500">
            Chargement...
          </p>
        ) : error ? (
          <p className="col-span-3 py-6 text-center text-xs text-red-400">
            {error}
          </p>
        ) : gifs.length === 0 ? (
          <p className="col-span-3 py-6 text-center text-xs text-zinc-500">
            Aucun résultat
          </p>
        ) : (
          gifs.map((gif) => (
            <div
              key={gif.id}
              className="group relative cursor-pointer"
              onMouseEnter={() => setHoveredGif(gif)}
              onMouseLeave={() => setHoveredGif(null)}
              onClick={() => {
                onSelect(gif.originalUrl ?? "");
                onClose();
              }}
            >
              <img
                src={gif.previewUrl}
                alt={gif.title}
                className="w-full rounded-lg transition-opacity group-hover:opacity-75"
                loading="lazy"
              />
              
              <div className="absolute inset-x-0 bottom-0 hidden rounded-b-lg bg-black/70 px-1 py-0.5 group-hover:block">
                <p className="truncate text-[9px] text-zinc-300">
                  @{gif.username}
                </p>
              </div>
            </div>
          ))
        )}
      </div>

      
      {hoveredGif && (
        <div className="mt-1 truncate text-[10px] text-zinc-500">
          {hoveredGif.title} —{" "}
          <a
            href={hoveredGif.source}
            target="_blank"
            rel="noopener noreferrer"
            className="underline hover:text-zinc-300"
            onClick={(e) => e.stopPropagation()}
          >
            Voir sur GIPHY
          </a>
        </div>
      )}

      
      <div className="mt-2 flex items-center justify-end gap-1 border-t border-zinc-800 pt-2">
        <span className="text-[10px] text-zinc-500">Powered by</span>
        <a
          href="https://giphy.com"
          target="_blank"
          rel="noopener noreferrer"
          className="text-[10px] font-bold text-zinc-400 hover:text-zinc-200"
        >
          GIPHY
        </a>
      </div>
    </div>
  );
}
