const DEFAULT_BACKEND_HOST = "localhost";
const BACKEND_PORT = 3000;

const isHttpProtocol = (protocol: string) =>
  protocol === "http:" || protocol === "https:";

const normalizeUrl = (url: string) => url.replace(/\/+$/, "");

const httpToWsUrl = (url: string) => {
  const normalized = normalizeUrl(url);

  if (normalized.startsWith("https://")) {
    return normalized.replace("https://", "wss://");
  }

  if (normalized.startsWith("http://")) {
    return normalized.replace("http://", "ws://");
  }

  return normalized;
};

export const resolveApiBaseUrl = (
  configuredUrl: string | undefined,
  protocol: string,
  hostname: string,
) => {
  if (configuredUrl) return normalizeUrl(configuredUrl);

  const apiProtocol = isHttpProtocol(protocol) ? protocol : "http:";
  const apiHost = isHttpProtocol(protocol) && hostname ? hostname : DEFAULT_BACKEND_HOST;

  return `${apiProtocol}//${apiHost}:${BACKEND_PORT}`;
};

export const resolveWsBaseUrl = (
  configuredUrl: string | undefined,
  protocol: string,
  hostname: string,
) => {
  if (configuredUrl) return httpToWsUrl(configuredUrl);

  const wsProtocol = protocol === "https:" ? "wss:" : "ws:";
  const wsHost = isHttpProtocol(protocol) && hostname ? hostname : DEFAULT_BACKEND_HOST;

  return `${wsProtocol}//${wsHost}:${BACKEND_PORT}`;
};

export const getRuntimeApiBaseUrl = () => {
  if (typeof window === "undefined") {
    return `http://${DEFAULT_BACKEND_HOST}:${BACKEND_PORT}`;
  }

  return resolveApiBaseUrl(
    process.env.NEXT_PUBLIC_API_URL,
    window.location.protocol,
    window.location.hostname,
  );
};

export const getRuntimeWsBaseUrl = () => {
  if (typeof window === "undefined") return null;

  return resolveWsBaseUrl(
    process.env.NEXT_PUBLIC_API_URL,
    window.location.protocol,
    window.location.hostname,
  );
};