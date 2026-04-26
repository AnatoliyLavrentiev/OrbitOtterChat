export type MemberRole = "OWNER" | "ADMIN" | "MEMBER" | "Owner" | "Admin" | "Member";
export type DisplayNameMode = "nickname" | "username";

export type Server = {
  id: string;
  name?: string;
};

export type Channel = {
  id: string;
  name: string;
  topic?: string | null;
  position?: number;
  server_id?: string | null;
};

export type ChatMessage = {
  id: string;
  author_id: string;
  content: string;
  pinned_at?: string | null;
  pinned_by?: string | null;
};

export type MessageReaction = {
  emoji: string;
  count: number;
  reacted: boolean;
};

export type Member = {
  user_id: string;
  role: MemberRole;
  username?: string;
  nickname?: string | null;
  avatar_url?: string | null;
  display_name_mode?: DisplayNameMode;
};

export type ServerBan = {
  server_id: string;
  user_id: string;
  banned_by: string;
  reason: string | null;
  expires_at: string | null;
  created_at: string;
  username?: string | null;
  nickname?: string | null;
  avatar_url?: string | null;
  display_name_mode?: DisplayNameMode | null;
};

export type MeData = {
  id?: string;
  email?: string;
  username?: string;
  nickname?: string | null;
  avatar_url?: string | null;
  display_name_mode?: DisplayNameMode;
  error?: string;
};

export type BlockedUser = {
  user_id: string;
  username?: string | null;
  nickname?: string | null;
  avatar_url?: string | null;
  display_name_mode?: DisplayNameMode | null;
  created_at: string;
};

type AuthHeader = Record<string, string> | undefined;

const toJson = async <T>(res: Response): Promise<T> => {
  return (await res.json()) as T;
};

const toJsonSafe = async <T>(res: Response): Promise<T> => {
  const raw = await res.text();
  if (!raw) return {} as T;
  try {
    return JSON.parse(raw) as T;
  } catch {
    return {} as T;
  }
};

const withAuth = (token: string): AuthHeader => ({ Authorization: `Bearer ${token}` });

export const signUp = async (
  apiUrl: string,
  payload: { email: string; username: string; password: string },
) => {
  const res = await fetch(`${apiUrl}/auth/signup`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });
  return { ok: res.ok, data: await toJson<{ access_token?: string; error?: string }>(res) };
};

export const login = async (
  apiUrl: string,
  payload: { email: string; password: string },
) => {
  const res = await fetch(`${apiUrl}/auth/login`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });
  return { ok: res.ok, data: await toJson<{ access_token?: string; error?: string }>(res) };
};

export const me = async (apiUrl: string, token: string) => {
  const res = await fetch(`${apiUrl}/me`, { headers: withAuth(token) });
  return { ok: res.ok, data: await toJson<MeData>(res) };
};

export const updateMe = async (
  apiUrl: string,
  token: string,
  payload: {
    email?: string;
    username?: string;
    nickname?: string;
    avatar_url?: string;
    display_name_mode?: DisplayNameMode;
  },
) => {
  const res = await fetch(`${apiUrl}/me`, {
    method: "PUT",
    headers: {
      "Content-Type": "application/json",
      ...withAuth(token),
    },
    body: JSON.stringify(payload),
  });
  return { ok: res.ok, data: await toJson<MeData>(res) };
};

export const uploadAvatar = async (apiUrl: string, token: string, file: File) => {
  const formData = new FormData();
  formData.append("avatar", file);
  const res = await fetch(`${apiUrl}/me/avatar`, {
    method: "POST",
    headers: withAuth(token),
    body: formData,
  });
  return { ok: res.ok, data: await toJson<MeData>(res) };
};

export const listServers = async (apiUrl: string, token: string) => {
  const res = await fetch(`${apiUrl}/servers`, { headers: withAuth(token) });
  return { ok: res.ok, data: await toJson<Server[]>(res) };
};

export const createServer = async (
  apiUrl: string,
  token: string,
  payload: {
    name: string;
    initial_channel_name?: string;
    initial_channel_description?: string;
  },
) => {
  const res = await fetch(`${apiUrl}/servers`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...withAuth(token),
    },
    body: JSON.stringify(payload),
  });
  return { ok: res.ok, data: await toJson<unknown>(res) };
};

export const joinByInvite = async (apiUrl: string, token: string, inviteCode: string) => {
  const res = await fetch(`${apiUrl}/servers/join`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...withAuth(token),
    },
    body: JSON.stringify({ invite_code: inviteCode }),
  });
  return { ok: res.ok, data: await toJson<string>(res) };
};

export const leaveServer = async (apiUrl: string, token: string, serverId: string) => {
  const res = await fetch(`${apiUrl}/servers/${serverId}/leave`, {
    method: "DELETE",
    headers: withAuth(token),
  });
  return { ok: res.ok };
};

export const createInvite = async (apiUrl: string, token: string, serverId: string) => {
  const res = await fetch(`${apiUrl}/servers/${serverId}/invites`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...withAuth(token),
    },
    body: JSON.stringify({}),
  });
  return { ok: res.ok, data: await toJson<{ invite_code?: string }>(res) };
};

export const listChannels = async (apiUrl: string, token: string, serverId: string) => {
  const res = await fetch(`${apiUrl}/servers/${serverId}/channels`, {
    headers: withAuth(token),
  });
  return { ok: res.ok, data: await toJson<Channel[]>(res) };
};

export const createChannel = async (
  apiUrl: string,
  token: string,
  serverId: string,
  name: string,
  topic?: string,
) => {
  const res = await fetch(`${apiUrl}/servers/${serverId}/channels`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...withAuth(token),
    },
    body: JSON.stringify({ name, topic }),
  });
  return { ok: res.ok };
};

export const updateChannel = async (
  apiUrl: string,
  token: string,
  channelId: string,
  payload: { name?: string; topic?: string | null; position?: number },
) => {
  const res = await fetch(`${apiUrl}/channels/${channelId}`, {
    method: "PUT",
    headers: {
      "Content-Type": "application/json",
      ...withAuth(token),
    },
    body: JSON.stringify(payload),
  });
  return { ok: res.ok };
};

export const deleteChannel = async (apiUrl: string, token: string, channelId: string) => {
  const res = await fetch(`${apiUrl}/channels/${channelId}`, {
    method: "DELETE",
    headers: withAuth(token),
  });
  return { ok: res.ok };
};

export const listMembers = async (apiUrl: string, token: string, serverId: string) => {
  const res = await fetch(`${apiUrl}/servers/${serverId}/members`, {
    headers: withAuth(token),
  });
  return { ok: res.ok, data: await toJson<Member[]>(res) };
};

export const kickMember = async (
  apiUrl: string,
  token: string,
  serverId: string,
  userId: string,
) => {
  const res = await fetch(`${apiUrl}/servers/${serverId}/members/${userId}`, {
    method: "DELETE",
    headers: withAuth(token),
  });
  return { ok: res.ok };
};

export const banMember = async (
  apiUrl: string,
  token: string,
  serverId: string,
  payload: { user_id: string; duration_hours?: number; reason?: string },
) => {
  const res = await fetch(`${apiUrl}/servers/${serverId}/bans`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...withAuth(token),
    },
    body: JSON.stringify(payload),
  });
  return { ok: res.ok };
};

export const listBans = async (apiUrl: string, token: string, serverId: string) => {
  const res = await fetch(`${apiUrl}/servers/${serverId}/bans`, {
    headers: withAuth(token),
  });
  return { ok: res.ok, data: await toJson<ServerBan[]>(res) };
};

export const unbanMember = async (
  apiUrl: string,
  token: string,
  serverId: string,
  userId: string,
) => {
  const res = await fetch(`${apiUrl}/servers/${serverId}/bans/${userId}`, {
    method: "DELETE",
    headers: withAuth(token),
  });
  return { ok: res.ok };
};

export const updateMemberRole = async (
  apiUrl: string,
  token: string,
  serverId: string,
  userId: string,
  role: "ADMIN" | "MEMBER",
) => {
  const res = await fetch(`${apiUrl}/servers/${serverId}/members/${userId}`, {
    method: "PUT",
    headers: {
      "Content-Type": "application/json",
      ...withAuth(token),
    },
    body: JSON.stringify({ role }),
  });
  return { ok: res.ok };
};

export const transferOwnership = async (
  apiUrl: string,
  token: string,
  serverId: string,
  newOwnerId: string,
) => {
  const res = await fetch(`${apiUrl}/servers/${serverId}/ownership`, {
    method: "PUT",
    headers: {
      "Content-Type": "application/json",
      ...withAuth(token),
    },
    body: JSON.stringify({ new_owner_id: newOwnerId }),
  });
  return { ok: res.ok };
};

export const listMessages = async (apiUrl: string, token: string, channelId: string) => {
  const res = await fetch(`${apiUrl}/channels/${channelId}/messages`, {
    headers: withAuth(token),
  });
  return { ok: res.ok, data: await toJson<ChatMessage[]>(res) };
};

export const sendMessage = async (
  apiUrl: string,
  token: string,
  channelId: string,
  content: string,
) => {
  const res = await fetch(`${apiUrl}/channels/${channelId}/messages`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...withAuth(token),
    },
    body: JSON.stringify({ content }),
  });
  return { ok: res.ok, data: await toJson<{ error?: string }>(res) };
};

export const uploadMessageFile = async (
  apiUrl: string,
  token: string,
  channelId: string,
  file: File,
) => {
  const formData = new FormData();
  formData.append("file", file);
  const res = await fetch(`${apiUrl}/channels/${channelId}/messages/file`, {
    method: "POST",
    headers: withAuth(token),
    body: formData,
  });
  return { ok: res.ok, data: await toJsonSafe<{ message?: ChatMessage; error?: string }>(res) };
};

export const updateMessage = async (
  apiUrl: string,
  token: string,
  messageId: string,
  content: string,
) => {
  const res = await fetch(`${apiUrl}/messages/${messageId}`, {
    method: "PUT",
    headers: {
      "Content-Type": "application/json",
      ...withAuth(token),
    },
    body: JSON.stringify({ content }),
  });
  return { ok: res.ok, data: await toJson<{ message?: ChatMessage }>(res) };
};

export const deleteMessage = async (apiUrl: string, token: string, messageId: string) => {
  const res = await fetch(`${apiUrl}/messages/${messageId}`, {
    method: "DELETE",
    headers: withAuth(token),
  });
  return { ok: res.ok };
};

export const pinMessage = async (apiUrl: string, token: string, messageId: string) => {
  const res = await fetch(`${apiUrl}/messages/${messageId}/pin`, {
    method: "PUT",
    headers: withAuth(token),
  });
  return { ok: res.ok, data: await toJsonSafe<{ message?: ChatMessage; error?: string }>(res) };
};

export const unpinMessage = async (apiUrl: string, token: string, messageId: string) => {
  const res = await fetch(`${apiUrl}/messages/${messageId}/pin`, {
    method: "DELETE",
    headers: withAuth(token),
  });
  return { ok: res.ok, data: await toJsonSafe<{ message?: ChatMessage; error?: string }>(res) };
};

export const createOrGetDm = async (
  apiUrl: string,
  token: string,
  targetUserId: string,
) => {
  const res = await fetch(`${apiUrl}/dms/${targetUserId}`, {
    method: "POST",
    headers: withAuth(token),
  });
  return { ok: res.ok, data: await toJson<Channel & { error?: string }>(res) };
};

export const listDms = async (apiUrl: string, token: string) => {
  const res = await fetch(`${apiUrl}/dms`, {
    headers: withAuth(token),
  });
  return { ok: res.ok, data: await toJson<Channel[]>(res) };
};

export const deleteDmHistory = async (
  apiUrl: string,
  token: string,
  channelId: string,
) => {
  const res = await fetch(`${apiUrl}/dms/channel/${channelId}`, {
    method: "DELETE",
    headers: withAuth(token),
  });
  return { ok: res.ok, data: await toJsonSafe<{ error?: string }>(res) };
};

export const listBlockedUsers = async (apiUrl: string, token: string) => {
  const res = await fetch(`${apiUrl}/blocks`, {
    headers: withAuth(token),
  });
  return { ok: res.ok, data: await toJson<BlockedUser[]>(res) };
};

export const blockUser = async (
  apiUrl: string,
  token: string,
  targetUserId: string,
) => {
  const res = await fetch(`${apiUrl}/blocks/${targetUserId}`, {
    method: "POST",
    headers: withAuth(token),
  });
  return { ok: res.ok, data: await toJsonSafe<{ error?: string }>(res) };
};

export const unblockUser = async (
  apiUrl: string,
  token: string,
  targetUserId: string,
) => {
  const res = await fetch(`${apiUrl}/blocks/${targetUserId}`, {
    method: "DELETE",
    headers: withAuth(token),
  });
  return { ok: res.ok, data: await toJsonSafe<{ error?: string }>(res) };
};

export const listMessageReactions = async (apiUrl: string, token: string, messageId: string) => {
  const res = await fetch(`${apiUrl}/messages/${messageId}/reactions`, {
    headers: withAuth(token),
  });
  return { ok: res.ok, data: await toJson<{ reactions?: MessageReaction[] }>(res) };
};

export const toggleMessageReaction = async (
  apiUrl: string,
  token: string,
  messageId: string,
  emoji: string,
) => {
  const res = await fetch(`${apiUrl}/messages/${messageId}/reactions`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...withAuth(token),
    },
    body: JSON.stringify({ emoji }),
  });
  return { ok: res.ok, data: await toJson<{ reactions?: MessageReaction[]; error?: string }>(res) };
};
