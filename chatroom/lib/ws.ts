import { getRuntimeWsBaseUrl } from "./runtimeEndpoints";

export type WsEvent =
  | {
      event: "presence_joined" | "presence_left";
      server_id: string;
      user_id: string;
      connected_users: string[];
    }
  | {
      event: "typing_start" | "typing_stop";
      server_id: string;
      channel_id: string;
      user_id: string;
    }
  | {
      event: "message_new";
      server_id: string | null;
      channel_id: string;
      message_id: string;
      author_id: string;
      content: string;
      dm_member_ids?: string[] | null;
    }
  | {
      event: "message_deleted";
      server_id: string | null;
      channel_id: string;
      message_id: string;
      deleted_by: string;
      dm_member_ids?: string[] | null;
    }
  | {
      event: "message_reactions_updated";
      server_id: string | null;
      channel_id: string;
      message_id: string;
      dm_member_ids?: string[] | null;
    }
  | {
      event: "status_updated";
      server_id: string;
      user_id: string;
      status: "online" | "away" | "invisible";
    }
  | {
      event: "server_ban_applied";
      server_id: string;
      user_id: string;
      banned_by: string;
      reason?: string | null;
      expires_at?: string | null;
    };

export const getWsBaseUrl = () => {
  return getRuntimeWsBaseUrl();
};

export const openChatSocket = (params: {
  token: string;
  serverId: string;
  channelId: string | null;
  onEvent: (event: WsEvent) => void;
  onClose?: () => void;
}) => {
  const base = getWsBaseUrl();
  if (!base) return null;
  const query = new URLSearchParams({
    token: params.token,
    server_id: params.serverId,
  });
  if (params.channelId) query.set("channel_id", params.channelId);

  const socket = new WebSocket(`${base}/ws?${query.toString()}`);
  socket.onmessage = (message) => {
    try {
      const event = JSON.parse(message.data) as WsEvent;
      params.onEvent(event);
    } catch {
      return;
    }
  };
  socket.onclose = () => params.onClose?.();
  return socket;
};

export const emitTyping = (
  socket: WebSocket | null,
  channelId: string | null,
  event: "typing_start" | "typing_stop",
) => {
  if (!socket || socket.readyState !== WebSocket.OPEN || !channelId) return;
  socket.send(JSON.stringify({ event, channel_id: channelId }));
};

export const emitStatus = (
  socket: WebSocket | null,
  status: "online" | "away" | "invisible",
) => {
  if (!socket || socket.readyState !== WebSocket.OPEN) return;
  socket.send(JSON.stringify({ event: "set_status", status }));
};
