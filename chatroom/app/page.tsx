"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  type BlockedUser,
  type Channel,
  type ChatMessage,
  type DisplayNameMode,
  type Member,
  type MessageReaction,
  type MemberRole,
  type Server,
  type ServerBan,
  banMember,
  createChannel,
  createInvite,
  createOrGetDm,
  createServer,
  deleteDmHistory,
  deleteChannel,
  deleteMessage,
  joinByInvite,
  kickMember,
  leaveServer,
  listBans,
  listChannels,
  listDms,
  listBlockedUsers,
  listMembers,
  listMessages,
  listMessageReactions,
  listServers,
  login,
  me,
  pinMessage,
  sendMessage,
  signUp,
  blockUser,
  toggleMessageReaction,
  transferOwnership,
  unbanMember,
  unblockUser,
  unpinMessage,
  updateChannel,
  updateMemberRole,
  updateMe,
  updateMessage,
  uploadAvatar,
  uploadMessageFile,
} from "../lib/api";
import { formatFileSize, parseFileMessage } from "../lib/fileMessage";
import { filterMessagesBySearchQuery } from "../lib/messageSearch";
import { getRuntimeApiBaseUrl } from "../lib/runtimeEndpoints";
import { emitStatus, emitTyping, openChatSocket, type WsEvent } from "../lib/ws";
import GifPicker from "./components/GifPicker";

const SESSION_COOKIE_NAME = "rtc_access_token";
const SESSION_COOKIE_MAX_AGE_SECONDS = 60 * 60 * 24 * 7;
const DM_CACHE_KEY_PREFIX = "rtc_dm_channels";
const KNOWN_USERS_CACHE_KEY_PREFIX = "rtc_known_users";

const readSessionToken = (): string | null => {
  if (typeof document === "undefined") return null;
  const cookies = document.cookie ? document.cookie.split("; ") : [];
  const key = `${SESSION_COOKIE_NAME}=`;
  const found = cookies.find((item) => item.startsWith(key));
  if (!found) return null;
  const raw = found.slice(key.length);
  if (!raw) return null;
  try {
    return decodeURIComponent(raw);
  } catch {
    return raw;
  }
};

const writeSessionToken = (token: string) => {
  if (typeof document === "undefined") return;
  document.cookie = `${SESSION_COOKIE_NAME}=${encodeURIComponent(token)}; Path=/; Max-Age=${SESSION_COOKIE_MAX_AGE_SECONDS}; SameSite=Lax`;
};

const clearSessionToken = () => {
  if (typeof document === "undefined") return;
  document.cookie = `${SESSION_COOKIE_NAME}=; Path=/; Max-Age=0; SameSite=Lax`;
};

type Presence = "online" | "away" | "invisible";
type ToastType = "success" | "error" | "info";
type Toast = { id: number; text: string; type: ToastType };
type Language = "en" | "fr";

const LANGUAGE_STORAGE_KEY = "rtc_language";

const I18N = {
  en: {
    language: "Language",
    english: "English",
    french: "French",
    realtimeTeamChat: "Realtime Team Chat",
    orbitOtterChat: "Orbit Otter Chat",
    welcomeDescription:
      "Smooth team conversations with otter-speed updates. Your orbit for channels, private chats, reactions, and live collaboration.",
    welcomeTagline: "Stay curious. Stay connected. Stay in orbit.",
    signIn: "Sign in",
    createAccount: "Create account",
    joinTheOrbit: "Join the orbit",
    welcomeBack: "Welcome back",
    authLeftSignupDescription:
      "Create your profile and start chatting with your crew in seconds.",
    authLeftSigninDescription:
      "Jump back into your channels and continue the conversation.",
    authSignupTitle: "Create account",
    authSigninTitle: "Sign in",
    authSignupDescription:
      "Set up your identity and launch your first conversation.",
    authSigninDescription: "Use your account to enter Orbit Otter Chat.",
    email: "Email",
    username: "Username",
    password: "Password",
    loading: "Loading...",
    enterChat: "Enter chat",
    backToWelcomeScreen: "Back to welcome screen",
    openSettings: "Open settings",
    selectServerFirst: "Select a server first",
    openProfileSettings: "Open profile settings",
    channels: "Channels",
    logout: "Logout",
    serverSettings: "Server settings",
    serverSettingsSubtitle: "Members, moderation and role management",
    close: "Close",
    profileSettings: "Profile settings",
    profileSettingsSubtitle: "Manage your account details",
    languageInSettings: "Interface language",
    inviteCode: "Invite code",
    createInvite: "Create invite",
    creating: "Creating...",
    leaveServer: "Leave server",
    ownerCannotLeave: "Owner cannot leave. Transfer ownership first.",
    inviteLabel: "Invite",
    createChannel: "Create channel",
    channelTitle: "Channel title",
    channelDescriptionOptional: "Channel description (optional)",
    add: "Add",
    adding: "Adding...",
    editSelectedChannel: "Edit selected channel",
    position: "Position",
    saveChannelChanges: "Save channel changes",
    deleteSelectedChannel: "Delete selected channel",
    onlineNow: "Online",
    membersTotal: "Members",
    status: "Status",
    serverChannels: "Server channels",
    directMessages: "Direct messages",
    chatWith: "Chat with",
    blockUser: "Block",
    unblockUser: "Unblock",
    blocked: "Blocked",
    deleteDmHistory: "Delete conversation",
    deletingConversation: "Deleting...",
    noChannelsYet: "No channels yet.",
    noDirectMessagesYet: "No direct messages yet.",
    noChannelSelected: "No channel selected",
    needChannelToChat: "You need a channel to chat.",
    createOneFromLeft: "Create one from the left panel using New channel.",
    askAdminCreateChannel: "Ask an owner or admin to create a channel for this server.",
    edit: "Edit",
    delete: "Delete",
    deleteModerate: "Delete (moderate)",
    save: "Save",
    cancel: "Cancel",
    addReaction: "Add reaction",
    pin: "Pin",
    unpin: "Unpin",
    pinned: "Pinned",
    pinnedMessages: "Pinned messages",
    searchMessages: "Search messages...",
    noSearchResults: "No matching messages",
    attachFile: "Attach file",
    uploadingFile: "Uploading...",
    writeMessage: "Write a message...",
    selectChannelToWrite: "Create or select a channel to write messages",
    noOneTyping: "No one is typing",
    selectChannelStartChat: "Select a channel to start chatting",
    you: "You",
    youTyping: "You are typing...",
    isTyping: "is typing...",
    areTyping: "are typing...",
    others: "others",
    presence: "Presence",
    currentStatus: "Current status",
    banOptions: "Ban options",
    banQuickDuration: "Quick duration",
    clearDuration: "Clear duration",
    currentDuration: "Current duration",
    banReasonOptional: "Ban reason (optional)",
    banDurationHint: "Duration in hours (empty = permanent)",
    bannedUsers: "Banned users",
    noReason: "No reason",
    noBannedUsers: "No banned users",
    until: "until",
    permanent: "permanent",
    unban: "Unban",
    members: "Members",
    role: "Role",
    mention: "Mention",
    dm: "DM",
    setAdmin: "Set admin",
    setMember: "Set member",
    transferOwnership: "Transfer ownership",
    kick: "Kick",
    tempBan: "Temp ban",
    permanentBan: "Permanent ban",
    noMembersLoaded: "No members loaded",
    owner: "Owner",
    admin: "Admin",
    member: "Member",
    online: "online",
    away: "away",
    invisible: "invisible",
    and: "and",
    account: "Account",
    addChangeAvatarHint: "Add or change avatar with a public image URL.",
    nickname: "Nickname",
    showNicknameInChat: "Show nickname in chat",
    showUsernameInChat: "Show username in chat",
    avatarUrl: "Avatar URL",
    uploadAvatarFromComputer: "Upload avatar from computer",
    uploadAvatar: "Upload avatar",
    userId: "User ID",
    unknown: "unknown",
    saving: "Saving...",
    saveProfile: "Save profile",
    createServer: "Create server",
    serverName: "Server name",
    firstChannelTitle: "First channel title (default: general)",
    firstChannelDescription: "First channel description (optional)",
    create: "Create",
    deleteChannelTitle: "Delete channel",
    deleteMessageTitle: "Delete message",
    cannotUndo: "This action cannot be undone.",
    deleting: "Deleting...",
    bannedFromServer: "You have been banned from this server.",
    bannedFromServerWithReason: "You have been banned from this server. Reason:",
    removedFromServer: "You no longer have access to this server.",
    dmBlockedByPolicy: "Direct messages are disabled because one user blocked the other.",
  },
  fr: {
    language: "Langue",
    english: "Anglais",
    french: "Français",
    realtimeTeamChat: "Chat d'équipe en temps réel",
    orbitOtterChat: "Orbit Otter Chat",
    welcomeDescription:
      "Des conversations fluides à la vitesse d'une loutre. Votre orbite pour les canaux, messages privés, réactions et collaboration en direct.",
    welcomeTagline: "Restez curieux. Restez connectés. Restez en orbite.",
    signIn: "Connexion",
    createAccount: "Créer un compte",
    joinTheOrbit: "Rejoindre l'orbite",
    welcomeBack: "Bon retour",
    authLeftSignupDescription:
      "Créez votre profil et commencez à discuter avec votre équipe en quelques secondes.",
    authLeftSigninDescription:
      "Revenez dans vos canaux et reprenez la conversation.",
    authSignupTitle: "Créer un compte",
    authSigninTitle: "Connexion",
    authSignupDescription:
      "Configurez votre identité et lancez votre première conversation.",
    authSigninDescription: "Utilisez votre compte pour entrer dans Orbit Otter Chat.",
    email: "Email",
    username: "Nom d'utilisateur",
    password: "Mot de passe",
    loading: "Chargement...",
    enterChat: "Entrer dans le chat",
    backToWelcomeScreen: "Retour à l'écran d'accueil",
    openSettings: "Ouvrir les paramètres",
    selectServerFirst: "Sélectionnez d'abord un serveur",
    openProfileSettings: "Ouvrir les paramètres du profil",
    channels: "Canaux",
    logout: "Déconnexion",
    serverSettings: "Paramètres du serveur",
    serverSettingsSubtitle: "Membres, modération et gestion des rôles",
    close: "Fermer",
    profileSettings: "Paramètres du profil",
    profileSettingsSubtitle: "Gérez les détails de votre compte",
    languageInSettings: "Langue de l'interface",
    inviteCode: "Code d'invitation",
    createInvite: "Créer une invitation",
    creating: "Création...",
    leaveServer: "Quitter le serveur",
    ownerCannotLeave: "Le propriétaire ne peut pas quitter. Transférez la propriété d'abord.",
    inviteLabel: "Invitation",
    createChannel: "Créer un canal",
    channelTitle: "Titre du canal",
    channelDescriptionOptional: "Description du canal (optionnelle)",
    add: "Ajouter",
    adding: "Ajout...",
    editSelectedChannel: "Modifier le canal sélectionné",
    position: "Position",
    saveChannelChanges: "Enregistrer les modifications",
    deleteSelectedChannel: "Supprimer le canal sélectionné",
    onlineNow: "En ligne",
    membersTotal: "Membres",
    status: "Statut",
    serverChannels: "Canaux du serveur",
    directMessages: "Messages privés",
    chatWith: "Chat avec",
    blockUser: "Bloquer",
    unblockUser: "Débloquer",
    blocked: "Bloqué",
    deleteDmHistory: "Supprimer la conversation",
    deletingConversation: "Suppression...",
    noChannelsYet: "Aucun canal pour le moment.",
    noDirectMessagesYet: "Aucun message privé pour le moment.",
    noChannelSelected: "Aucun canal sélectionné",
    needChannelToChat: "Vous avez besoin d'un canal pour discuter.",
    createOneFromLeft: "Créez-en un depuis le panneau de gauche avec Nouveau canal.",
    askAdminCreateChannel: "Demandez à un propriétaire ou un admin de créer un canal pour ce serveur.",
    edit: "Modifier",
    delete: "Supprimer",
    deleteModerate: "Supprimer (modération)",
    save: "Enregistrer",
    cancel: "Annuler",
    addReaction: "Ajouter une réaction",
    pin: "Épingler",
    unpin: "Désépingler",
    pinned: "Épinglé",
    pinnedMessages: "Messages épinglés",
    searchMessages: "Rechercher des messages...",
    noSearchResults: "Aucun message correspondant",
    attachFile: "Joindre un fichier",
    uploadingFile: "Envoi...",
    writeMessage: "Écrire un message...",
    selectChannelToWrite: "Créez ou sélectionnez un canal pour écrire des messages",
    noOneTyping: "Personne n'écrit",
    selectChannelStartChat: "Sélectionnez un canal pour commencer à discuter",
    you: "Vous",
    youTyping: "Vous écrivez...",
    isTyping: "écrit...",
    areTyping: "écrivent...",
    others: "autres",
    presence: "Présence",
    currentStatus: "Statut actuel",
    banOptions: "Options de bannissement",
    banQuickDuration: "Durée rapide",
    clearDuration: "Effacer la durée",
    currentDuration: "Durée actuelle",
    banReasonOptional: "Raison du bannissement (optionnelle)",
    banDurationHint: "Durée en heures (vide = permanent)",
    bannedUsers: "Utilisateurs bannis",
    noReason: "Aucune raison",
    noBannedUsers: "Aucun utilisateur banni",
    until: "jusqu'à",
    permanent: "permanent",
    unban: "Débannir",
    members: "Membres",
    role: "Rôle",
    mention: "Mentionner",
    dm: "MP",
    setAdmin: "Définir admin",
    setMember: "Définir membre",
    transferOwnership: "Transférer la propriété",
    kick: "Expulser",
    tempBan: "Ban temp.",
    permanentBan: "Ban permanent",
    noMembersLoaded: "Aucun membre chargé",
    owner: "Propriétaire",
    admin: "Admin",
    member: "Membre",
    online: "en ligne",
    away: "absent",
    invisible: "invisible",
    and: "et",
    account: "Compte",
    addChangeAvatarHint: "Ajoutez ou modifiez l'avatar avec une URL d'image publique.",
    nickname: "Surnom",
    showNicknameInChat: "Afficher le surnom dans le chat",
    showUsernameInChat: "Afficher le nom d'utilisateur dans le chat",
    avatarUrl: "URL de l'avatar",
    uploadAvatarFromComputer: "Téléverser un avatar depuis l'ordinateur",
    uploadAvatar: "Téléverser l'avatar",
    userId: "ID utilisateur",
    unknown: "inconnu",
    saving: "Enregistrement...",
    saveProfile: "Enregistrer le profil",
    createServer: "Créer un serveur",
    serverName: "Nom du serveur",
    firstChannelTitle: "Titre du premier canal (défaut : general)",
    firstChannelDescription: "Description du premier canal (optionnelle)",
    create: "Créer",
    deleteChannelTitle: "Supprimer le canal",
    deleteMessageTitle: "Supprimer le message",
    cannotUndo: "Cette action est irréversible.",
    deleting: "Suppression...",
    bannedFromServer: "Vous avez été banni de ce serveur.",
    bannedFromServerWithReason: "Vous avez été banni de ce serveur. Raison :",
    removedFromServer: "Vous n'avez plus accès à ce serveur.",
    dmBlockedByPolicy: "Les messages privés sont désactivés car un utilisateur a bloqué l'autre.",
  },
} as const;

type TranslationKey = keyof (typeof I18N)["en"];

const shortUser = (id: string) => `User ${id.slice(0, 8)}`;

const resolveAvatarUrl = (apiUrl: string, raw?: string | null) => {
  if (!raw) return null;
  if (raw.startsWith("http://") || raw.startsWith("https://")) return raw;
  return `${apiUrl}${raw}`;
};

const normalizeRole = (role: MemberRole | null | undefined): "OWNER" | "ADMIN" | "MEMBER" | null => {
  if (!role) return null;
  const v = role.toUpperCase();
  if (v === "OWNER" || v === "ADMIN" || v === "MEMBER") return v;
  return null;
};

const canModerate = (role: MemberRole | null) => {
  const normalized = normalizeRole(role);
  return normalized === "OWNER" || normalized === "ADMIN";
};

const canTarget = (
  actor: MemberRole | null,
  target: MemberRole,
  isSelf: boolean,
): boolean => {
  const actorRole = normalizeRole(actor);
  const targetRole = normalizeRole(target);
  if (isSelf) return false;
  if (!actorRole || !targetRole) return false;
  if (actorRole === "OWNER") return targetRole !== "OWNER";
  if (actorRole === "ADMIN") return targetRole === "MEMBER";
  return false;
};

const asMentionHandle = (value?: string | null) => {
  const trimmed = (value ?? "").trim();
  if (!trimmed) return null;
  if (!/^[A-Za-z0-9_-]{3,}$/.test(trimmed)) return null;
  return trimmed;
};

const escapeRegex = (value: string) => value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

const memberDisplayName = (member?: Member | null) => {
  if (!member) return null;
  if (member.display_name_mode === "username") {
    return member.username || member.nickname || null;
  }
  return member.nickname || member.username || null;
};

const renderMessageContent = (content: string, apiUrl: string) => {
  if (content.startsWith("gif::")) {
    const gifUrl = content.slice("gif::".length);
    return (
      <img
        src={gifUrl}
        alt="GIF"
        className="max-w-xs rounded-lg"
        loading="lazy"
      />
    );
  }
  const file = parseFileMessage(content);
  if (file) {
    const fileUrl = resolveAvatarUrl(apiUrl, file.url) ?? file.url;
    const isImage = file.mime.startsWith("image/");
    return (
      <a
        href={fileUrl}
        target="_blank"
        rel="noopener noreferrer"
        className="block max-w-xs text-left"
      >
        {isImage ? (
          <img
            src={fileUrl}
            alt={file.filename}
            className="mb-2 max-h-56 rounded-lg object-contain"
            loading="lazy"
          />
        ) : null}
        <span className="block font-medium text-sky-200">{file.filename}</span>
        <span className="block text-xs text-zinc-400">
          {file.mime} · {formatFileSize(file.size)}
        </span>
      </a>
    );
  }
  const parts = content.split(/(\s+)/);
  return (
    <>
      {parts.map((part, idx) => {
        if (/^@[A-Za-z0-9_-]{3,}$/.test(part)) {
          return (
            <span key={`${part}-${idx}`} className="text-sky-300">
              {part}
            </span>
          );
        }
        return <span key={`${part}-${idx}`}>{part}</span>;
      })}
    </>
  );
};

const REACTION_OPTIONS = [
  "👍",
  "❤️",
  "😂",
  "🔥",
  "👏",
  "🎉",
  "🤝",
  "🙏",
  "😮",
  "😢",
  "😡",
  "✅",
];

const DM_ID_PATTERN = /[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-5][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}/g;

const BTN_PRIMARY =
  "rounded-lg border border-cyan-400/30 bg-cyan-500/10 px-2 py-1 text-xs font-medium text-cyan-200 transition hover:bg-cyan-500/20 disabled:opacity-50";
const BTN_DANGER =
  "rounded-lg border border-rose-400/30 bg-rose-500/10 px-2 py-1 text-xs font-medium text-rose-200 transition hover:bg-rose-500/20 disabled:opacity-50";
const BTN_MUTED =
  "rounded-lg border border-zinc-600 bg-zinc-800/70 px-2 py-1 text-xs text-zinc-200 transition hover:bg-zinc-700 disabled:opacity-50";

const PixelOtterMascot = ({ className = "" }: { className?: string }) => (
  <svg
    viewBox="0 0 1024 1024"
    className={className}
    role="img"
    aria-label="Static otter astronaut with twinkling stars"
  >
    <image
      href="/otter-astronaut.png"
      x="0"
      y="0"
      width="1024"
      height="1024"
      preserveAspectRatio="xMidYMid slice"
      style={{ imageRendering: "pixelated" }}
    />

    <g opacity="0.9">
      <circle cx="116" cy="126" r="5" fill="#8be9fd">
        <animate attributeName="opacity" values="0.05;1;0.05" dur="2s" repeatCount="indefinite" />
        <animate attributeName="r" values="4;6.6;4" dur="2s" repeatCount="indefinite" />
      </circle>
      <circle cx="198" cy="92" r="4" fill="#facc15">
        <animate attributeName="opacity" values="1;0.04;1" dur="2.6s" repeatCount="indefinite" />
        <animate attributeName="r" values="3.4;5.8;3.4" dur="2.6s" repeatCount="indefinite" />
      </circle>
      <circle cx="640" cy="102" r="6" fill="#e879f9">
        <animate attributeName="opacity" values="0.06;1;0.06" dur="1.7s" repeatCount="indefinite" />
        <animate attributeName="r" values="5;8;5" dur="1.7s" repeatCount="indefinite" />
      </circle>
      <circle cx="848" cy="166" r="5" fill="#7dd3fc">
        <animate attributeName="opacity" values="1;0.03;1" dur="2.8s" repeatCount="indefinite" />
        <animate attributeName="r" values="4;6.7;4" dur="2.8s" repeatCount="indefinite" />
      </circle>
      <circle cx="876" cy="534" r="5" fill="#fde68a">
        <animate attributeName="opacity" values="0.04;1;0.04" dur="2.2s" repeatCount="indefinite" />
        <animate attributeName="r" values="4;6.4;4" dur="2.2s" repeatCount="indefinite" />
      </circle>
      <circle cx="146" cy="582" r="6" fill="#c4b5fd">
        <animate attributeName="opacity" values="1;0.04;1" dur="1.9s" repeatCount="indefinite" />
        <animate attributeName="r" values="5;8;5" dur="1.9s" repeatCount="indefinite" />
      </circle>
      <circle cx="812" cy="852" r="6" fill="#f0abfc">
        <animate attributeName="opacity" values="0.05;1;0.05" dur="2.4s" repeatCount="indefinite" />
        <animate attributeName="r" values="5;8.2;5" dur="2.4s" repeatCount="indefinite" />
      </circle>
      <circle cx="240" cy="856" r="5" fill="#93c5fd">
        <animate attributeName="opacity" values="1;0.03;1" dur="2.9s" repeatCount="indefinite" />
        <animate attributeName="r" values="4;6.8;4" dur="2.9s" repeatCount="indefinite" />
      </circle>
    </g>
  </svg>
);

export default function App() {
  const API_URL = getRuntimeApiBaseUrl();
  const initialSessionToken = readSessionToken();

  const [isLoggedIn, setIsLoggedIn] = useState(Boolean(initialSessionToken));
  const [showAuthForm, setShowAuthForm] = useState(false);
  const [isSignup, setIsSignup] = useState(false);
  const [email, setEmail] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [authError, setAuthError] = useState("");
  const [authLoading, setAuthLoading] = useState(false);

  const [accessToken, setAccessToken] = useState<string | null>(initialSessionToken);
  const [currentUserId, setCurrentUserId] = useState<string | null>(null);

  const [profileEmail, setProfileEmail] = useState("");
  const [profileUsername, setProfileUsername] = useState("");
  const [profileNickname, setProfileNickname] = useState("");
  const [profileAvatarUrl, setProfileAvatarUrl] = useState("");
  const [profileDisplayNameMode, setProfileDisplayNameMode] =
    useState<DisplayNameMode>("nickname");
  const [profileAvatarFile, setProfileAvatarFile] = useState<File | null>(null);
  const [profileLoading, setProfileLoading] = useState(false);
  const [profileError, setProfileError] = useState("");
  const [profileSuccess, setProfileSuccess] = useState("");

  const [servers, setServers] = useState<Server[]>([]);
  const [channels, setChannels] = useState<Channel[]>([]);
  const [dmChannels, setDmChannels] = useState<Channel[]>([]);
  const [members, setMembers] = useState<Member[]>([]);
  const [knownUsersById, setKnownUsersById] = useState<Record<string, Member>>({});
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [reactionsByMessage, setReactionsByMessage] = useState<Record<string, MessageReaction[]>>(
    {},
  );
  const [reactionPickerByMessage, setReactionPickerByMessage] = useState<Record<string, boolean>>(
    {},
  );
  const [dmPeerNameByChannelId, setDmPeerNameByChannelId] = useState<Record<string, string>>({});
  const [blockedUsers, setBlockedUsers] = useState<BlockedUser[]>([]);
  const [bans, setBans] = useState<ServerBan[]>([]);

  const [selectedServerId, setSelectedServerId] = useState<string | null>(null);
  const [selectedChannelId, setSelectedChannelId] = useState<string | null>(null);
  const [channelUnreadCount, setChannelUnreadCount] = useState<Record<string, number>>({});
  const [channelMentionCount, setChannelMentionCount] = useState<Record<string, number>>({});

  const [newMessage, setNewMessage] = useState("");
  const [messageSearchQuery, setMessageSearchQuery] = useState("");
  const [newChannelName, setNewChannelName] = useState("");
  const [newChannelDescription, setNewChannelDescription] = useState("");
  const [channelEditName, setChannelEditName] = useState("");
  const [channelEditTopic, setChannelEditTopic] = useState("");
  const [channelEditPosition, setChannelEditPosition] = useState("");
  const [inviteCodeInput, setInviteCodeInput] = useState("");
  const [lastInviteCode, setLastInviteCode] = useState<string | null>(null);

  const [chatError, setChatError] = useState("");
  const [chatLoading, setChatLoading] = useState(false);
  const [joinLoading, setJoinLoading] = useState(false);
  const [createInviteLoading, setCreateInviteLoading] = useState(false);
  const [createChannelLoading, setCreateChannelLoading] = useState(false);
  const [updateChannelLoading, setUpdateChannelLoading] = useState(false);
  const [createServerLoading, setCreateServerLoading] = useState(false);
  const [deleteChannelLoading, setDeleteChannelLoading] = useState(false);
  const [deleteMessageLoading, setDeleteMessageLoading] = useState(false);
  const [deleteDmLoading, setDeleteDmLoading] = useState(false);
  const [toasts, setToasts] = useState<Toast[]>([]);

  const [onlineUsers, setOnlineUsers] = useState<string[]>([]);
  const [typingUsers, setTypingUsers] = useState<string[]>([]);
  const [presenceMap, setPresenceMap] = useState<Record<string, Presence>>({});
  const [myPresence, setMyPresence] = useState<Presence>("online");
  const [language, setLanguage] = useState<Language>("en");

  const [showGifPicker, setShowGifPicker] = useState(false);
  const [fileUploadLoading, setFileUploadLoading] = useState(false);

  const [isCreateServerModalOpen, setIsCreateServerModalOpen] = useState(false);
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [isProfileSettingsOpen, setIsProfileSettingsOpen] = useState(false);
  const [newServerName, setNewServerName] = useState("");
  const [newServerInitialChannelName, setNewServerInitialChannelName] = useState("general");
  const [newServerInitialChannelDescription, setNewServerInitialChannelDescription] =
    useState("");
  const [confirmDeleteChannelOpen, setConfirmDeleteChannelOpen] = useState(false);
  const [confirmDeleteMessageId, setConfirmDeleteMessageId] = useState<string | null>(null);

  const [editingMessageId, setEditingMessageId] = useState<string | null>(null);
  const [editingMessageContent, setEditingMessageContent] = useState("");

  const [banReason, setBanReason] = useState("");
  const [banDurationHours, setBanDurationHours] = useState("");

  const wsRef = useRef<WebSocket | null>(null);
  const typingTimersRef = useRef<Record<string, ReturnType<typeof setTimeout>>>({});
  const settingsCloseBtnRef = useRef<HTMLButtonElement | null>(null);
  const profileCloseBtnRef = useRef<HTMLButtonElement | null>(null);
  const createServerInputRef = useRef<HTMLInputElement | null>(null);
  const deleteChannelCancelRef = useRef<HTMLButtonElement | null>(null);
  const deleteMessageCancelRef = useRef<HTMLButtonElement | null>(null);
  const messageFileInputRef = useRef<HTMLInputElement | null>(null);

  const pushToast = useCallback((text: string, type: ToastType = "info") => {
    const id = Date.now() + Math.floor(Math.random() * 10000);
    setToasts((prev) => [...prev, { id, text, type }]);
  }, []);
  const t = useCallback((key: TranslationKey) => I18N[language][key], [language]);

  const currentMember = useMemo(
    () => members.find((m) => m.user_id === currentUserId) ?? null,
    [members, currentUserId],
  );
  const currentUserMentionHandles = useMemo(() => {
    const values = [
      currentMember?.nickname ?? null,
      currentMember?.username ?? null,
      profileNickname,
      profileUsername,
    ]
      .map((value) => asMentionHandle(value))
      .filter((value): value is string => Boolean(value));
    return Array.from(new Set(values.map((value) => value.toLowerCase())));
  }, [currentMember?.nickname, currentMember?.username, profileNickname, profileUsername]);
  const profilePreviewAvatar = useMemo(
    () => resolveAvatarUrl(API_URL, profileAvatarUrl.trim() || null),
    [API_URL, profileAvatarUrl],
  );
  const currentRole = currentMember?.role ?? null;
  const currentRoleNormalized = normalizeRole(currentRole);
  const canAdminChannels = currentRoleNormalized === "OWNER" || currentRoleNormalized === "ADMIN";
  const canOwnerManage = currentRoleNormalized === "OWNER";
  const canManageMembers = canModerate(currentRole);
  const isOwnerInSelectedServer = currentRoleNormalized === "OWNER";
  const allChannels = useMemo(() => {
    const map = new Map<string, Channel>();
    channels.forEach((channel) => map.set(channel.id, channel));
    dmChannels.forEach((channel) => map.set(channel.id, channel));
    return Array.from(map.values());
  }, [channels, dmChannels]);
  const selectedChannel = useMemo(
    () => allChannels.find((c) => c.id === selectedChannelId) ?? null,
    [allChannels, selectedChannelId],
  );
  const filteredMessages = useMemo(
    () => filterMessagesBySearchQuery(messages, messageSearchQuery),
    [messageSearchQuery, messages],
  );
  const pinnedMessages = useMemo(
    () => messages.filter((message) => Boolean(message.pinned_at)),
    [messages],
  );
  const blockedUserIds = useMemo(
    () => new Set(blockedUsers.map((item) => item.user_id)),
    [blockedUsers],
  );
  const membersById = useMemo(
    () => new Map(members.map((m) => [m.user_id, m])),
    [members],
  );
  const getKnownMember = useCallback(
    (userId: string) => membersById.get(userId) ?? knownUsersById[userId] ?? null,
    [knownUsersById, membersById],
  );
  const getMemberDisplayName = useCallback(
    (userId: string) => {
      if (userId === currentUserId) return t("you");
      const member = getKnownMember(userId);
      return memberDisplayName(member) || shortUser(userId);
    },
    [currentUserId, getKnownMember, t],
  );
  const roleLabel = useCallback(
    (role: MemberRole) => {
      const normalized = normalizeRole(role);
      if (normalized === "OWNER") return t("owner");
      if (normalized === "ADMIN") return t("admin");
      return t("member");
    },
    [t],
  );
  const presenceLabel = useCallback(
    (value: Presence) => {
      if (value === "online") return t("online");
      if (value === "away") return t("away");
      return t("invisible");
    },
    [t],
  );
  const dmCacheKey = useMemo(
    () => (currentUserId ? `${DM_CACHE_KEY_PREFIX}:${currentUserId}` : null),
    [currentUserId],
  );
  const knownUsersCacheKey = useMemo(
    () => (currentUserId ? `${KNOWN_USERS_CACHE_KEY_PREFIX}:${currentUserId}` : null),
    [currentUserId],
  );

  const getDmPeerIdFromChannel = useCallback(
    (channel: Channel | null) => {
      if (!channel || !channel.name.startsWith("dm-")) return null;
      const matches = channel.name.match(DM_ID_PATTERN) ?? [];
      if (matches.length === 0) return null;
      return matches.find((id) => id !== currentUserId) ?? matches[0];
    },
    [currentUserId],
  );
  const selectedDmPeerId = useMemo(
    () =>
      selectedChannel && selectedChannel.server_id === null
        ? getDmPeerIdFromChannel(selectedChannel)
        : null,
    [getDmPeerIdFromChannel, selectedChannel],
  );
  const isSelectedDmBlockedByMe = useMemo(
    () => Boolean(selectedDmPeerId && blockedUserIds.has(selectedDmPeerId)),
    [blockedUserIds, selectedDmPeerId],
  );

  const getChannelLabel = useCallback(
    (channel: Channel) => {
      const explicitDmName = dmPeerNameByChannelId[channel.id];
      if (explicitDmName) return explicitDmName;
      const dmPeerId = getDmPeerIdFromChannel(channel);
      if (dmPeerId) return getMemberDisplayName(dmPeerId);
      return `# ${channel.name}`;
    },
    [dmPeerNameByChannelId, getDmPeerIdFromChannel, getMemberDisplayName],
  );
  const isMentioningCurrentUser = useCallback(
    (content: string) => {
      if (!content || currentUserMentionHandles.length === 0) return false;
      return currentUserMentionHandles.some((handle) => {
        const pattern = new RegExp(`(^|\\s)@${escapeRegex(handle)}(?=\\s|$|[.,!?;:])`, "i");
        return pattern.test(content);
      });
    },
    [currentUserMentionHandles],
  );

  const restoreDmChannelsFromCache = useCallback(() => {
    if (typeof window === "undefined" || !dmCacheKey) return;
    try {
      const raw = window.localStorage.getItem(dmCacheKey);
      if (!raw) return;
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) return;
      const restored = parsed.filter(
        (item: unknown): item is Channel =>
          Boolean(
            item &&
              typeof item === "object" &&
              "id" in item &&
              "name" in item &&
              typeof (item as { id?: unknown }).id === "string" &&
              typeof (item as { name?: unknown }).name === "string",
          ),
      );
      if (restored.length > 0) setDmChannels(restored);
    } catch {
      return;
    }
  }, [dmCacheKey]);

  const loadServers = useCallback(async () => {
    if (!accessToken) return;
    const result = await listServers(API_URL, accessToken);
    if (!result.ok) throw new Error("load servers failed");
    const parsed = Array.isArray(result.data) ? result.data : [];
    setServers(parsed);
    if (
      selectedServerId &&
      !parsed.some((server) => server.id === selectedServerId)
    ) {
      setIsSettingsOpen(false);
      setSelectedServerId(null);
      setSelectedChannelId(null);
      setChannels([]);
      setMembers([]);
      setMessages([]);
      setBans([]);
      pushToast(t("removedFromServer"), "error");
      return;
    }
    if (parsed.length > 0 && !selectedServerId) setSelectedServerId(parsed[0].id);
  }, [API_URL, accessToken, pushToast, selectedServerId, t]);

  const loadChannels = useCallback(
    async (serverId: string) => {
      if (!accessToken) return;
      const result = await listChannels(API_URL, accessToken, serverId);
      if (!result.ok) throw new Error("load channels failed");
      const parsed = Array.isArray(result.data) ? result.data : [];
      setChannels(parsed);
      if (parsed.length > 0) {
        setSelectedChannelId((prev) => {
          if (prev && parsed.some((channel) => channel.id === prev)) return prev;
          return parsed[0].id;
        });
      } else {
        setSelectedChannelId(null);
        setMessages([]);
        setReactionsByMessage({});
      }
    },
    [API_URL, accessToken],
  );

  const loadDms = useCallback(async () => {
    if (!accessToken) return;
    const result = await listDms(API_URL, accessToken);
    if (!result.ok) throw new Error("load dms failed");
    const parsed = Array.isArray(result.data) ? result.data : [];
    setDmChannels(parsed);
  }, [API_URL, accessToken]);

  const loadBlocked = useCallback(async () => {
    if (!accessToken) return;
    const result = await listBlockedUsers(API_URL, accessToken);
    if (!result.ok) throw new Error("load blocks failed");
    const parsed = Array.isArray(result.data) ? result.data : [];
    setBlockedUsers(parsed);
    setKnownUsersById((prev) => {
      const next = { ...prev };
      parsed.forEach((blocked) => {
        next[blocked.user_id] = {
          ...(next[blocked.user_id] ?? { user_id: blocked.user_id, role: "MEMBER" }),
          username: blocked.username ?? next[blocked.user_id]?.username,
          nickname: blocked.nickname ?? next[blocked.user_id]?.nickname ?? null,
          avatar_url: blocked.avatar_url ?? next[blocked.user_id]?.avatar_url ?? null,
          display_name_mode:
            blocked.display_name_mode ?? next[blocked.user_id]?.display_name_mode ?? "nickname",
        };
      });
      return next;
    });
  }, [API_URL, accessToken]);

  const loadMembers = useCallback(
    async (serverId: string) => {
      if (!accessToken) return;
      const result = await listMembers(API_URL, accessToken, serverId);
      if (!result.ok) throw new Error("load members failed");
      const parsed = Array.isArray(result.data) ? result.data : [];
      setMembers(parsed);
      setKnownUsersById((prev) => {
        const next = { ...prev };
        parsed.forEach((member) => {
          next[member.user_id] = {
            ...(next[member.user_id] ?? {}),
            ...member,
          };
        });
        return next;
      });
    },
    [API_URL, accessToken],
  );

  const loadBans = useCallback(
    async (serverId: string) => {
      if (!accessToken || !canManageMembers) {
        setBans([]);
        return;
      }
      const result = await listBans(API_URL, accessToken, serverId);
      if (!result.ok) throw new Error("load bans failed");
      const parsed = Array.isArray(result.data) ? result.data : [];
      setBans(parsed);
      setKnownUsersById((prev) => {
        const next = { ...prev };
        parsed.forEach((ban) => {
          const existing = next[ban.user_id];
          next[ban.user_id] = {
            user_id: ban.user_id,
            role: existing?.role ?? "MEMBER",
            username: ban.username ?? existing?.username,
            nickname: ban.nickname ?? existing?.nickname ?? null,
            avatar_url: ban.avatar_url ?? existing?.avatar_url ?? null,
            display_name_mode:
              ban.display_name_mode ?? existing?.display_name_mode ?? "nickname",
          };
        });
        return next;
      });
    },
    [API_URL, accessToken, canManageMembers],
  );

  const handleAuth = async (e: React.FormEvent) => {
    e.preventDefault();
    setAuthError("");
    setAuthLoading(true);
    try {
      const result = isSignup
        ? await signUp(API_URL, { email, username, password })
        : await login(API_URL, { email, password });

      if (!result.ok || !result.data.access_token) {
        setAuthError(result.data.error || "Authentication failed");
        return;
      }

      setAccessToken(result.data.access_token);
      writeSessionToken(result.data.access_token);
      setIsLoggedIn(true);
      setPassword("");
      setUsername("");
      pushToast(isSignup ? "Account created" : "Signed in", "success");
    } catch {
      setAuthError("Authentication server is unreachable");
    } finally {
      setAuthLoading(false);
    }
  };

  useEffect(() => {
    if (!isLoggedIn || !accessToken) return;
    loadServers().catch(() => setChatError("Failed to load servers"));
    restoreDmChannelsFromCache();
    loadDms().catch(() => setChatError("Failed to load direct messages"));
    loadBlocked().catch(() => setChatError("Failed to load blocked users"));
    me(API_URL, accessToken)
      .then((result) => {
        if (!result.ok) {
          clearSessionToken();
          setAccessToken(null);
          setIsLoggedIn(false);
          setShowAuthForm(true);
          setCurrentUserId(null);
          return;
        }
        setCurrentUserId(result.data.id ?? null);
        setProfileEmail((result.data as { email?: string }).email ?? "");
        setProfileUsername((result.data as { username?: string }).username ?? "");
        setProfileNickname((result.data as { nickname?: string | null }).nickname ?? "");
        setProfileAvatarUrl((result.data as { avatar_url?: string | null }).avatar_url ?? "");
        setProfileDisplayNameMode(
          (result.data as { display_name_mode?: DisplayNameMode }).display_name_mode ?? "nickname",
        );
        if (result.data.id) {
          setKnownUsersById((prev) => ({
            ...prev,
            [result.data.id as string]: {
              ...(prev[result.data.id as string] ?? {}),
              user_id: result.data.id as string,
              username: (result.data as { username?: string }).username,
              nickname: (result.data as { nickname?: string | null }).nickname,
              avatar_url: (result.data as { avatar_url?: string | null }).avatar_url,
              display_name_mode:
                (result.data as { display_name_mode?: DisplayNameMode }).display_name_mode ??
                "nickname",
              role: "MEMBER",
            },
          }));
        }
      })
      .catch(() => setCurrentUserId(null));
  }, [
    API_URL,
    accessToken,
    isLoggedIn,
    loadBlocked,
    loadDms,
    loadServers,
    restoreDmChannelsFromCache,
  ]);

  useEffect(() => {
    if (typeof window === "undefined" || !dmCacheKey) return;
    try {
      window.localStorage.setItem(dmCacheKey, JSON.stringify(dmChannels));
    } catch {
      return;
    }
  }, [dmCacheKey, dmChannels]);

  useEffect(() => {
    if (typeof window === "undefined" || !knownUsersCacheKey) return;
    try {
      const raw = window.localStorage.getItem(knownUsersCacheKey);
      if (!raw) return;
      const parsed = JSON.parse(raw) as Record<string, Member>;
      if (!parsed || typeof parsed !== "object") return;
      setKnownUsersById((prev) => ({ ...parsed, ...prev }));
    } catch {
      return;
    }
  }, [knownUsersCacheKey]);

  useEffect(() => {
    if (typeof window === "undefined" || !knownUsersCacheKey) return;
    try {
      window.localStorage.setItem(knownUsersCacheKey, JSON.stringify(knownUsersById));
    } catch {
      return;
    }
  }, [knownUsersById, knownUsersCacheKey]);

  useEffect(() => {
    if (typeof window === "undefined") return;
    const stored = window.localStorage.getItem(LANGUAGE_STORAGE_KEY);
    if (stored === "en" || stored === "fr") {
      setLanguage(stored);
      return;
    }
    if (window.navigator.language.toLowerCase().startsWith("fr")) {
      setLanguage("fr");
    }
  }, []);

  useEffect(() => {
    if (typeof window === "undefined") return;
    window.localStorage.setItem(LANGUAGE_STORAGE_KEY, language);
  }, [language]);

  useEffect(() => {
    if (!isLoggedIn || !accessToken) return;
    const timer = setInterval(() => {
      loadDms().catch(() => {});
    }, 8000);
    return () => clearInterval(timer);
  }, [accessToken, isLoggedIn, loadDms]);

  useEffect(() => {
    if (!isLoggedIn || !accessToken) return;
    const timer = setInterval(() => {
      loadServers().catch(() => {});
    }, 8000);
    return () => clearInterval(timer);
  }, [accessToken, isLoggedIn, loadServers]);

  useEffect(() => {
    if (!selectedServerId) return;
    setChatError("");
    setPresenceMap({});
    loadChannels(selectedServerId).catch(() => setChatError("Failed to load channels"));
    loadMembers(selectedServerId).catch(() => setChatError("Failed to load members"));
  }, [loadChannels, loadMembers, selectedServerId]);

  useEffect(() => {
    if (!selectedChannelId) return;
    setMessageSearchQuery("");
    setChannelUnreadCount((prev) => ({ ...prev, [selectedChannelId]: 0 }));
    setChannelMentionCount((prev) => ({ ...prev, [selectedChannelId]: 0 }));
  }, [selectedChannelId]);

  useEffect(() => {
    const channelIds = new Set(channels.map((channel) => channel.id));
    setChannelUnreadCount((prev) =>
      Object.fromEntries(
        Object.entries(prev).filter(([channelId]) => channelIds.has(channelId)),
      ),
    );
    setChannelMentionCount((prev) =>
      Object.fromEntries(
        Object.entries(prev).filter(([channelId]) => channelIds.has(channelId)),
      ),
    );
  }, [channels]);

  useEffect(() => {
    if (!isLoggedIn || !accessToken || !selectedServerId) return;
    const timer = setInterval(() => {
      loadChannels(selectedServerId).catch(() => {});
      loadMembers(selectedServerId).catch(() => {});
    }, 5000);
    return () => clearInterval(timer);
  }, [accessToken, isLoggedIn, loadChannels, loadMembers, selectedServerId]);

  useEffect(() => {
    if (!selectedServerId) return;
    loadBans(selectedServerId).catch(() => setChatError("Failed to load bans"));
  }, [loadBans, selectedServerId]);

  useEffect(() => {
    if (!selectedChannelId || !accessToken) {
      setMessages([]);
      setReactionsByMessage({});
      return;
    }
    listMessages(API_URL, accessToken, selectedChannelId)
      .then((result) => {
        if (!result.ok) throw new Error("messages failed");
        setMessages(Array.isArray(result.data) ? result.data : []);
      })
      .catch(() => setChatError("Failed to load messages"));
  }, [API_URL, accessToken, selectedChannelId]);

  useEffect(() => {
    if (!accessToken || messages.length === 0) {
      setReactionsByMessage({});
      return;
    }

    Promise.all(
      messages.map(async (message) => {
        const result = await listMessageReactions(API_URL, accessToken, message.id);
        return [
          message.id,
          result.ok && Array.isArray(result.data.reactions) ? result.data.reactions : [],
        ] as const;
      }),
    )
      .then((pairs) => {
        setReactionsByMessage(Object.fromEntries(pairs));
      })
      .catch(() => setChatError("Failed to load reactions"));
  }, [API_URL, accessToken, messages]);

  useEffect(() => {
    if (!selectedChannel) {
      setChannelEditName("");
      setChannelEditTopic("");
      setChannelEditPosition("");
      return;
    }
    setChannelEditName(selectedChannel.name);
    setChannelEditTopic(selectedChannel.topic ?? "");
    setChannelEditPosition(
      typeof selectedChannel.position === "number" ? String(selectedChannel.position) : "0",
    );
  }, [selectedChannel]);

  useEffect(() => {
    if (!accessToken || !selectedServerId) return;
    wsRef.current?.close();
    const socket = openChatSocket({
      token: accessToken,
      serverId: selectedServerId,
      channelId: null,
      onEvent: (parsed: WsEvent) => {
        if (parsed.event === "presence_joined" || parsed.event === "presence_left") {
          setOnlineUsers(parsed.connected_users);
          return;
        }
        if (parsed.event === "status_updated") {
          setPresenceMap((prev) => ({ ...prev, [parsed.user_id]: parsed.status }));
          return;
        }
        if (parsed.event === "server_ban_applied") {
          const reason = parsed.reason?.trim();
          const hasExpiry = Boolean(parsed.expires_at);
          const untilText = hasExpiry
            ? (() => {
                const date = new Date(parsed.expires_at as string);
                if (Number.isNaN(date.getTime())) return `${t("until")} ${parsed.expires_at}`;
                return `${t("until")} ${new Intl.DateTimeFormat(
                  language === "fr" ? "fr-FR" : "en-US",
                  {
                    dateStyle: "medium",
                    timeStyle: "short",
                  },
                ).format(date)}`;
              })()
            : t("permanent");
          pushToast(
            reason
              ? `${t("bannedFromServerWithReason")} ${reason} · ${untilText}`
              : `${t("bannedFromServer")} (${untilText})`,
            "error",
          );
          if (parsed.server_id === selectedServerId) {
            setIsSettingsOpen(false);
            setSelectedServerId(null);
            setSelectedChannelId(null);
            setChannels([]);
            setMembers([]);
            setMessages([]);
            setBans([]);
            loadServers().catch(() => {});
          }
          return;
        }
        if (parsed.event === "typing_start") {
          if (parsed.channel_id !== selectedChannelId) return;
          if (typingTimersRef.current[parsed.user_id]) {
            clearTimeout(typingTimersRef.current[parsed.user_id]);
          }
          typingTimersRef.current[parsed.user_id] = setTimeout(() => {
            setTypingUsers((prev) => prev.filter((id) => id !== parsed.user_id));
            delete typingTimersRef.current[parsed.user_id];
          }, 2500);
          setTypingUsers((prev) =>
            prev.includes(parsed.user_id) ? prev : [...prev, parsed.user_id],
          );
          return;
        }
        if (parsed.event === "typing_stop") {
          if (parsed.channel_id !== selectedChannelId) return;
          if (typingTimersRef.current[parsed.user_id]) {
            clearTimeout(typingTimersRef.current[parsed.user_id]);
            delete typingTimersRef.current[parsed.user_id];
          }
          setTypingUsers((prev) => prev.filter((id) => id !== parsed.user_id));
          return;
        }
        if (parsed.event === "message_new") {
          const isDmEvent = parsed.server_id === null;
          if (isDmEvent) {
            const dmPartnerId =
              (parsed.dm_member_ids ?? []).find((id) => id !== currentUserId) ?? parsed.author_id;
            const dmName = getMemberDisplayName(dmPartnerId);
            setDmChannels((prev) => {
              if (prev.some((channel) => channel.id === parsed.channel_id)) return prev;
              return [
                ...prev,
                {
                  id: parsed.channel_id,
                  name: `dm-${dmPartnerId}`,
                  topic: null,
                  position: 0,
                  server_id: null,
                },
              ];
            });
            setDmPeerNameByChannelId((prev) => ({
              ...prev,
              [parsed.channel_id]: dmName,
            }));
            if (selectedChannelId !== parsed.channel_id) {
              pushToast(`New direct message from ${dmName}`, "info");
            }
          }
          const isServerChannelEvent = parsed.server_id !== null;
          const isDifferentChannel = parsed.channel_id !== selectedChannelId;
          const isOwnMessage = parsed.author_id === currentUserId;
          if (isServerChannelEvent && isDifferentChannel && !isOwnMessage) {
            setChannelUnreadCount((prev) => ({
              ...prev,
              [parsed.channel_id]: (prev[parsed.channel_id] ?? 0) + 1,
            }));
            if (isMentioningCurrentUser(parsed.content)) {
              setChannelMentionCount((prev) => ({
                ...prev,
                [parsed.channel_id]: (prev[parsed.channel_id] ?? 0) + 1,
              }));
            }
          }
          if (parsed.channel_id !== selectedChannelId) return;
          setMessages((prev) => {
            if (prev.some((m) => m.id === parsed.message_id)) return prev;
            return [
              ...prev,
              {
                id: parsed.message_id,
                author_id: parsed.author_id,
                content: parsed.content,
              },
            ];
          });
          setReactionsByMessage((prev) => ({ ...prev, [parsed.message_id]: [] }));
          return;
        }
        if (parsed.event === "message_deleted") {
          if (parsed.channel_id !== selectedChannelId) return;
          setMessages((prev) => prev.filter((m) => m.id !== parsed.message_id));
          setReactionsByMessage((prev) => {
            const next = { ...prev };
            delete next[parsed.message_id];
            return next;
          });
          return;
        }
        if (parsed.event === "message_reactions_updated") {
          if (parsed.channel_id !== selectedChannelId || !accessToken) return;
          listMessageReactions(API_URL, accessToken, parsed.message_id)
            .then((result) => {
              if (!result.ok) return;
              setReactionsByMessage((prev) => ({
                ...prev,
                [parsed.message_id]: Array.isArray(result.data.reactions)
                  ? result.data.reactions
                  : [],
              }));
            })
            .catch(() => setChatError("Failed to load reactions"));
        }
      },
      onClose: () => {
        if (wsRef.current === socket) wsRef.current = null;
      },
    });

    wsRef.current = socket;
    return () => {
      Object.values(typingTimersRef.current).forEach(clearTimeout);
      typingTimersRef.current = {};
      setTypingUsers([]);
      socket?.close();
      if (wsRef.current === socket) wsRef.current = null;
    };
  }, [
    API_URL,
    accessToken,
    currentUserId,
    getMemberDisplayName,
    isMentioningCurrentUser,
    language,
    loadServers,
    pushToast,
    selectedChannelId,
    selectedServerId,
    t,
  ]);

  useEffect(() => {
    if (!chatError) return;
    pushToast(chatError, "error");
    setChatError("");
  }, [chatError, pushToast]);

  useEffect(() => {
    if (toasts.length === 0) return;
    const timers = toasts.map((toast) =>
      setTimeout(() => {
        setToasts((prev) => prev.filter((item) => item.id !== toast.id));
      }, 3500),
    );
    return () => timers.forEach(clearTimeout);
  }, [toasts]);

  useEffect(() => {
    if (isSettingsOpen) settingsCloseBtnRef.current?.focus();
  }, [isSettingsOpen]);

  useEffect(() => {
    if (isProfileSettingsOpen) profileCloseBtnRef.current?.focus();
  }, [isProfileSettingsOpen]);

  useEffect(() => {
    if (isCreateServerModalOpen) createServerInputRef.current?.focus();
  }, [isCreateServerModalOpen]);

  useEffect(() => {
    if (confirmDeleteChannelOpen) deleteChannelCancelRef.current?.focus();
  }, [confirmDeleteChannelOpen]);

  useEffect(() => {
    if (confirmDeleteMessageId) deleteMessageCancelRef.current?.focus();
  }, [confirmDeleteMessageId]);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      if (showGifPicker) {
        setShowGifPicker(false);
        return;
      }
      if (confirmDeleteMessageId) {
        setConfirmDeleteMessageId(null);
        return;
      }
      if (confirmDeleteChannelOpen) {
        setConfirmDeleteChannelOpen(false);
        return;
      }
      if (isCreateServerModalOpen) {
        setIsCreateServerModalOpen(false);
        return;
      }
      if (isProfileSettingsOpen) {
        setIsProfileSettingsOpen(false);
        return;
      }
      if (isSettingsOpen) {
        setIsSettingsOpen(false);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [
    showGifPicker,
    confirmDeleteChannelOpen,
    confirmDeleteMessageId,
    isCreateServerModalOpen,
    isProfileSettingsOpen,
    isSettingsOpen,
  ]);

  const handleSendTyping = (event: "typing_start" | "typing_stop") => {
    emitTyping(wsRef.current, selectedChannelId, event);
  };

  const typingText = useMemo(() => {
    if (!selectedChannelId) return t("selectChannelStartChat");
    if (typingUsers.length === 0) return t("noOneTyping");
    const names = typingUsers.map(getMemberDisplayName);
    if (names.length === 1) {
      return names[0] === t("you") ? t("youTyping") : `${names[0]} ${t("isTyping")}`;
    }
    if (names.length === 2) {
      return `${names[0]} ${t("and")} ${names[1]} ${t("areTyping")}`;
    }
    return `${names[0]}, ${names[1]} ${t("and")} ${names.length - 2} ${t("others")} ${t("areTyping")}`;
  }, [getMemberDisplayName, selectedChannelId, t, typingUsers]);

  const handleSetPresence = (status: Presence) => {
    setMyPresence(status);
    emitStatus(wsRef.current, status);
  };

  const handleProfileUpdate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!accessToken) return;
    setProfileError("");
    setProfileSuccess("");
    setProfileLoading(true);
    try {
      const result = await updateMe(API_URL, accessToken, {
        email: profileEmail,
        username: profileUsername,
        nickname: profileNickname,
        avatar_url: profileAvatarUrl,
        display_name_mode: profileDisplayNameMode,
      });
      if (!result.ok) {
        const message = result.data.error ?? "Unable to update profile";
        setProfileError(message);
        setChatError(message);
        return;
      }
      setProfileEmail(result.data.email ?? profileEmail);
      setProfileUsername(result.data.username ?? profileUsername);
      setProfileNickname(result.data.nickname ?? "");
      setProfileAvatarUrl(result.data.avatar_url ?? "");
      setProfileDisplayNameMode(result.data.display_name_mode ?? profileDisplayNameMode);
      if (currentUserId) {
        setMembers((prev) =>
          prev.map((member) =>
            member.user_id === currentUserId
              ? {
                  ...member,
                  username: result.data.username ?? member.username,
                  nickname: result.data.nickname ?? member.nickname ?? null,
                  avatar_url: result.data.avatar_url ?? member.avatar_url ?? null,
                  display_name_mode:
                    result.data.display_name_mode ?? member.display_name_mode ?? "nickname",
                }
              : member,
          ),
        );
        setKnownUsersById((prev) => ({
          ...prev,
          [currentUserId]: {
            ...(prev[currentUserId] ?? { user_id: currentUserId, role: "MEMBER" }),
            username: result.data.username ?? prev[currentUserId]?.username,
            nickname: result.data.nickname ?? prev[currentUserId]?.nickname ?? null,
            avatar_url: result.data.avatar_url ?? prev[currentUserId]?.avatar_url ?? null,
            display_name_mode:
              result.data.display_name_mode ??
              prev[currentUserId]?.display_name_mode ??
              "nickname",
          },
        }));
      }
      if (selectedServerId) {
        await loadMembers(selectedServerId);
      }
      setProfileSuccess("Profile saved");
      pushToast("Profile saved", "success");
      setChatError("");
      setIsProfileSettingsOpen(false);
    } catch {
      setProfileError("Profile service is unreachable");
      setChatError("Profile service is unreachable");
    } finally {
      setProfileLoading(false);
    }
  };

  const handleAvatarUpload = async () => {
    if (!accessToken || !profileAvatarFile) return;
    setProfileError("");
    setProfileSuccess("");
    setProfileLoading(true);
    const result = await uploadAvatar(API_URL, accessToken, profileAvatarFile);
    setProfileLoading(false);
    if (!result.ok) {
      const message = result.data.error ?? "Unable to upload avatar";
      setProfileError(message);
      setChatError(message);
      return;
    }
    setProfileAvatarUrl(result.data.avatar_url ?? "");
    if (currentUserId) {
      setMembers((prev) =>
        prev.map((member) =>
          member.user_id === currentUserId
            ? { ...member, avatar_url: result.data.avatar_url ?? member.avatar_url ?? null }
            : member,
        ),
      );
      setKnownUsersById((prev) => ({
        ...prev,
        [currentUserId]: {
          ...(prev[currentUserId] ?? { user_id: currentUserId, role: "MEMBER" }),
          avatar_url: result.data.avatar_url ?? prev[currentUserId]?.avatar_url ?? null,
        },
      }));
    }
    if (selectedServerId) {
      await loadMembers(selectedServerId);
    }
    setProfileAvatarFile(null);
    setProfileSuccess("Avatar uploaded");
    pushToast("Avatar uploaded", "success");
    setChatError("");
  };

  const handleSendMessage = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newMessage.trim() || !selectedChannelId || !accessToken) return;
    setChatLoading(true);
    try {
      const result = await sendMessage(
        API_URL,
        accessToken,
        selectedChannelId,
        newMessage.trim(),
      );
      if (!result.ok) {
        setChatError(result.data.error ?? "Unable to send message");
        return;
      }
      handleSendTyping("typing_stop");
      setNewMessage("");
    } catch {
      setChatError("Message service is unreachable");
    } finally {
      setChatLoading(false);
    }
  };

  const handleGifSelect = async (gifUrl: string) => {
    if (!selectedChannelId || !accessToken) return;
    setShowGifPicker(false);
    setChatLoading(true);
    try {
      const result = await sendMessage(
        API_URL,
        accessToken,
        selectedChannelId,
        `gif::${gifUrl}`,
      );
      if (!result.ok) {
        setChatError("Unable to send GIF");
      }
    } catch {
      setChatError("Message service is unreachable");
    } finally {
      setChatLoading(false);
    }
  };

  const handleMessageFileUpload = async (file: File | null) => {
    if (!file || !selectedChannelId || !accessToken) return;
    setFileUploadLoading(true);
    setChatError("");
    try {
      const result = await uploadMessageFile(API_URL, accessToken, selectedChannelId, file);
      if (!result.ok) {
        setChatError(result.data.error ?? "Unable to upload file");
        return;
      }
      pushToast("File uploaded", "success");
    } catch {
      setChatError("File upload service is unreachable");
    } finally {
      setFileUploadLoading(false);
      if (messageFileInputRef.current) {
        messageFileInputRef.current.value = "";
      }
    }
  };

  const beginEditMessage = (message: ChatMessage) => {
    setEditingMessageId(message.id);
    setEditingMessageContent(message.content);
  };

  const handleUpdateMessage = async () => {
    if (!editingMessageId || !editingMessageContent.trim() || !accessToken) return;
    const result = await updateMessage(
      API_URL,
      accessToken,
      editingMessageId,
      editingMessageContent.trim(),
    );
    if (!result.ok) {
      setChatError("Unable to edit message");
      return;
    }
    setMessages((prev) =>
      prev.map((msg) =>
        msg.id === editingMessageId ? { ...msg, content: editingMessageContent.trim() } : msg,
      ),
    );
    pushToast("Message updated", "success");
    setEditingMessageId(null);
    setEditingMessageContent("");
  };

  const handleTogglePinMessage = async (message: ChatMessage) => {
    if (!accessToken) return;
    const result = message.pinned_at
      ? await unpinMessage(API_URL, accessToken, message.id)
      : await pinMessage(API_URL, accessToken, message.id);
    if (!result.ok || !result.data.message) {
      setChatError(result.data.error ?? "Unable to update pinned message");
      return;
    }
    setMessages((prev) =>
      prev.map((item) => (item.id === message.id ? result.data.message! : item)),
    );
    pushToast(message.pinned_at ? t("unpin") : t("pin"), "success");
  };

  const openCreateServerModal = () => {
    setNewServerName("");
    setNewServerInitialChannelName("general");
    setNewServerInitialChannelDescription("");
    setIsCreateServerModalOpen(true);
  };

  const submitCreateServer = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newServerName.trim() || !accessToken) return;
    setCreateServerLoading(true);
    const result = await createServer(API_URL, accessToken, {
      name: newServerName.trim(),
      initial_channel_name: newServerInitialChannelName.trim()
        ? newServerInitialChannelName.trim()
        : undefined,
      initial_channel_description: newServerInitialChannelDescription.trim()
        ? newServerInitialChannelDescription.trim()
        : undefined,
    });
    setCreateServerLoading(false);
    if (!result.ok) {
      setChatError("Unable to create server");
      return;
    }
    setIsCreateServerModalOpen(false);
    pushToast("Server created", "success");
    await loadServers();
  };

  const joinServerByInvite = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!inviteCodeInput.trim() || !accessToken) return;
    setJoinLoading(true);
    const result = await joinByInvite(API_URL, accessToken, inviteCodeInput.trim());
    setJoinLoading(false);
    if (!result.ok) {
      setChatError("Unable to join server");
      return;
    }
    setInviteCodeInput("");
    pushToast("Joined server", "success");
    await loadServers();
    if (result.data) setSelectedServerId(result.data);
  };

  const handleCreateInviteCode = async () => {
    if (!selectedServerId || !accessToken) return;
    setCreateInviteLoading(true);
    const result = await createInvite(API_URL, accessToken, selectedServerId);
    setCreateInviteLoading(false);
    if (!result.ok) {
      setChatError("Unable to create invite");
      return;
    }
    setLastInviteCode(result.data.invite_code ?? null);
    pushToast("Invite created", "success");
  };

  const handleLeaveCurrentServer = async () => {
    if (!selectedServerId || !accessToken) return;
    if (isOwnerInSelectedServer) {
      setChatError("Owner cannot leave the server. Transfer ownership first.");
      return;
    }
    const result = await leaveServer(API_URL, accessToken, selectedServerId);
    if (!result.ok) {
      setChatError("Unable to leave server");
      return;
    }
    setSelectedServerId(null);
    setSelectedChannelId(null);
    setChannels([]);
    setMessages([]);
    setMembers([]);
    setBans([]);
    pushToast("Left server", "success");
    await loadServers();
  };

  const handleCreateChannel = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedServerId || !newChannelName.trim() || !accessToken) return;
    setCreateChannelLoading(true);
    try {
      const result = await createChannel(
        API_URL,
        accessToken,
        selectedServerId,
        newChannelName.trim(),
        newChannelDescription.trim() ? newChannelDescription.trim() : undefined,
      );
      if (!result.ok) {
        setChatError("Only owner/admin can create channels");
        return;
      }
      setChatError("");
      setNewChannelName("");
      setNewChannelDescription("");
      pushToast("Channel created", "success");
      await loadChannels(selectedServerId);
    } catch {
      setChatError("Channel service is unreachable");
    } finally {
      setCreateChannelLoading(false);
    }
  };

  const handleUpdateChannel = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedChannelId || !channelEditName.trim() || !accessToken) return;
    const parsedPosition = Number(channelEditPosition);
    if (!Number.isInteger(parsedPosition) || parsedPosition < 0) {
      setChatError("Position must be a non-negative integer");
      return;
    }
    setUpdateChannelLoading(true);
    const result = await updateChannel(API_URL, accessToken, selectedChannelId, {
      name: channelEditName.trim(),
      topic: channelEditTopic.trim() ? channelEditTopic.trim() : null,
      position: parsedPosition,
    });
    setUpdateChannelLoading(false);
    if (!result.ok) {
      setChatError("Unable to update channel");
      return;
    }
    setChatError("");
    pushToast("Channel updated", "success");
    if (selectedServerId) await loadChannels(selectedServerId);
  };

  const confirmChannelDelete = () => setConfirmDeleteChannelOpen(true);

  const handleDeleteChannel = async () => {
    if (!selectedChannelId || !accessToken) return;
    setDeleteChannelLoading(true);
    const result = await deleteChannel(API_URL, accessToken, selectedChannelId);
    setDeleteChannelLoading(false);
    if (!result.ok) {
      setChatError("Unable to delete channel");
      return;
    }
    setConfirmDeleteChannelOpen(false);
    pushToast("Channel deleted", "success");
    if (selectedServerId) await loadChannels(selectedServerId);
  };

  const requestDeleteMessage = (messageId: string) => setConfirmDeleteMessageId(messageId);

  const handleDeleteMessage = async () => {
    if (!confirmDeleteMessageId || !accessToken) return;
    setDeleteMessageLoading(true);
    const result = await deleteMessage(API_URL, accessToken, confirmDeleteMessageId);
    setDeleteMessageLoading(false);
    if (!result.ok) {
      setChatError("Unable to delete message");
      return;
    }
    const deletedId = confirmDeleteMessageId;
    setMessages((prev) => prev.filter((m) => m.id !== confirmDeleteMessageId));
    setReactionsByMessage((prev) => {
      const next = { ...prev };
      delete next[deletedId];
      return next;
    });
    setConfirmDeleteMessageId(null);
    pushToast("Message deleted", "success");
  };

  const handleToggleReaction = async (messageId: string, emoji: string) => {
    if (!accessToken) return;
    const result = await toggleMessageReaction(API_URL, accessToken, messageId, emoji);
    if (!result.ok) {
      setChatError(result.data.error ?? "Unable to toggle reaction");
      return;
    }
    setReactionsByMessage((prev) => ({
      ...prev,
      [messageId]: Array.isArray(result.data.reactions) ? result.data.reactions : [],
    }));
  };

  const handleUpdateRole = async (
    userId: string,
    role: "ADMIN" | "MEMBER",
  ) => {
    if (!selectedServerId || !accessToken) return;
    const result = await updateMemberRole(API_URL, accessToken, selectedServerId, userId, role);
    if (!result.ok) {
      setChatError("Unable to update role");
      return;
    }
    await loadMembers(selectedServerId);
  };

  const handleTransferOwnership = async (newOwnerId: string) => {
    if (!selectedServerId || !accessToken) return;
    const result = await transferOwnership(API_URL, accessToken, selectedServerId, newOwnerId);
    if (!result.ok) {
      setChatError("Unable to transfer ownership");
      return;
    }
    await loadMembers(selectedServerId);
  };

  const handleKick = async (targetUserId: string) => {
    if (!selectedServerId || !accessToken) return;
    const result = await kickMember(API_URL, accessToken, selectedServerId, targetUserId);
    if (!result.ok) {
      setChatError("Unable to kick member");
      return;
    }
    await loadMembers(selectedServerId);
  };

  const handleTemporaryBan = async (targetUserId: string) => {
    if (!selectedServerId || !accessToken) return;
    const duration = Number(banDurationHours);
    if (!Number.isFinite(duration) || duration <= 0) {
      setChatError("Set a valid temporary ban duration in hours");
      return;
    }
    const payload = {
      user_id: targetUserId,
      duration_hours: duration,
      reason: banReason.trim() || undefined,
    };
    const result = await banMember(API_URL, accessToken, selectedServerId, payload);
    if (!result.ok) {
      setChatError("Unable to apply temporary ban");
      return;
    }
    setChatError("");
    setBanReason("");
    setBanDurationHours("");
    await loadMembers(selectedServerId);
    await loadBans(selectedServerId);
  };

  const handlePermanentBan = async (targetUserId: string) => {
    if (!selectedServerId || !accessToken) return;
    const payload = {
      user_id: targetUserId,
      reason: banReason.trim() || undefined,
    };
    const result = await banMember(API_URL, accessToken, selectedServerId, payload);
    if (!result.ok) {
      setChatError("Unable to apply permanent ban");
      return;
    }
    setChatError("");
    setBanReason("");
    setBanDurationHours("");
    await loadMembers(selectedServerId);
    await loadBans(selectedServerId);
  };

  const handleUnban = async (targetUserId: string) => {
    if (!selectedServerId || !accessToken) return;
    const result = await unbanMember(API_URL, accessToken, selectedServerId, targetUserId);
    if (!result.ok) {
      setChatError("Unable to unban member");
      return;
    }
    await loadBans(selectedServerId);
  };

  const handleBlockUser = async (targetUserId: string) => {
    if (!accessToken) return;
    const result = await blockUser(API_URL, accessToken, targetUserId);
    if (!result.ok) {
      setChatError(result.data.error ?? t("dmBlockedByPolicy"));
      return;
    }
    await loadBlocked();
    pushToast(t("blocked"), "success");
  };

  const handleUnblockUser = async (targetUserId: string) => {
    if (!accessToken) return;
    const result = await unblockUser(API_URL, accessToken, targetUserId);
    if (!result.ok) {
      setChatError(result.data.error ?? "Unable to unblock user");
      return;
    }
    await loadBlocked();
    pushToast(t("unblockUser"), "success");
  };

  const handleDeleteSelectedDmHistory = async () => {
    if (!accessToken || !selectedChannelId || !selectedChannel || selectedChannel.server_id !== null)
      return;
    setDeleteDmLoading(true);
    const result = await deleteDmHistory(API_URL, accessToken, selectedChannelId);
    setDeleteDmLoading(false);
    if (!result.ok) {
      setChatError(result.data.error ?? "Unable to delete conversation");
      return;
    }
    setMessages([]);
    setReactionsByMessage({});
    pushToast(t("deleteDmHistory"), "success");
  };

  const handleOpenDirectMessage = async (
    targetUserId: string,
    preferredName?: string,
    memberSnapshot?: Member | null,
  ) => {
    if (!accessToken) return;
    if (blockedUserIds.has(targetUserId)) {
      setChatError(t("dmBlockedByPolicy"));
      return;
    }
    const result = await createOrGetDm(API_URL, accessToken, targetUserId);
    if (!result.ok || !result.data.id) {
      setChatError(result.data.error ?? "Unable to open direct message");
      return;
    }

    const dmChannel = {
      id: result.data.id,
      name: result.data.name ?? `dm-${targetUserId}`,
      topic: result.data.topic ?? null,
      position: result.data.position ?? 0,
      server_id: result.data.server_id ?? null,
    };

    setDmChannels((prev) => {
      if (prev.some((channel) => channel.id === dmChannel.id)) return prev;
      return [...prev, dmChannel];
    });
    setDmPeerNameByChannelId((prev) => ({
      ...prev,
      [dmChannel.id]: preferredName || getMemberDisplayName(targetUserId),
    }));
    if (memberSnapshot) {
      setKnownUsersById((prev) => ({
        ...prev,
        [targetUserId]: {
          ...(prev[targetUserId] ?? { user_id: targetUserId, role: "MEMBER" }),
          ...memberSnapshot,
        },
      }));
    }
    setSelectedChannelId(dmChannel.id);
    pushToast("Direct message opened", "success");
  };

  const getMentionHandleForMember = useCallback((member: Member) => {
    return asMentionHandle(member.nickname) ?? asMentionHandle(member.username) ?? null;
  }, []);

  const activeMentionQuery = useMemo(() => {
    const match = newMessage.match(/(?:^|\s)@([A-Za-z0-9_-]*)$/);
    if (!match) return null;
    return match[1].toLowerCase();
  }, [newMessage]);

  const mentionSuggestions = useMemo(() => {
    if (activeMentionQuery === null) return [];
    if (!selectedChannel || !selectedChannel.server_id) return [];
    return members
      .filter((member) => member.user_id !== currentUserId)
      .map((member) => {
        const handle = getMentionHandleForMember(member);
        if (!handle) return null;
        const display = memberDisplayName(member) || shortUser(member.user_id);
        return { userId: member.user_id, handle, display };
      })
      .filter(
        (
          item,
        ): item is {
          userId: string;
          handle: string;
          display: string;
        } => Boolean(item),
      )
      .filter((item) => item.handle.toLowerCase().includes(activeMentionQuery))
      .slice(0, 7);
  }, [
    activeMentionQuery,
    currentUserId,
    getMentionHandleForMember,
    members,
    selectedChannel,
  ]);

  const insertMention = (handle: string) => {
    setNewMessage((prev) => {
      if (/(?:^|\s)@[A-Za-z0-9_-]*$/.test(prev)) {
        return prev.replace(/(^|\s)@[A-Za-z0-9_-]*$/, `$1@${handle} `);
      }
      const mention = `@${handle}`;
      return prev ? `${prev} ${mention} ` : `${mention} `;
    });
  };

  const insertEmoji = (emoji: string) => setNewMessage((prev) => `${prev}${emoji}`);

  const canDeleteMessage = (message: ChatMessage) =>
    message.author_id === currentUserId ||
    currentRoleNormalized === "OWNER" ||
    currentRoleNormalized === "ADMIN";

  const canPinMessage = (message: ChatMessage) =>
    selectedChannel?.server_id === null || canDeleteMessage(message);

  const logout = () => {
    clearSessionToken();
    if (typeof window !== "undefined" && dmCacheKey) {
      window.localStorage.removeItem(dmCacheKey);
    }
    wsRef.current?.close();
    wsRef.current = null;
    setIsLoggedIn(false);
    setAccessToken(null);
    setCurrentUserId(null);
    setSelectedServerId(null);
    setSelectedChannelId(null);
    setServers([]);
    setChannels([]);
    setDmChannels([]);
    setBlockedUsers([]);
    setMembers([]);
    setMessages([]);
    setBans([]);
    setOnlineUsers([]);
    setTypingUsers([]);
    setPresenceMap({});
  };

  if (!isLoggedIn) {
    if (!showAuthForm) {
      return (
        <div className="relative flex min-h-screen items-center justify-center overflow-hidden bg-[#050816] px-4 py-10 text-zinc-100">
          <div className="absolute inset-0 bg-[radial-gradient(circle_at_15%_20%,rgba(34,211,238,0.24),transparent_40%),radial-gradient(circle_at_80%_0%,rgba(59,130,246,0.22),transparent_36%),radial-gradient(circle_at_75%_80%,rgba(16,185,129,0.18),transparent_34%)]" />
          <div className="relative w-full max-w-5xl rounded-[2rem] border border-cyan-300/20 bg-zinc-900/70 p-6 backdrop-blur-xl md:p-10">
            <div className="mb-5 flex items-center justify-end gap-2">
              <span className="text-xs uppercase tracking-[0.2em] text-zinc-400">{t("language")}</span>
              <button
                type="button"
                onClick={() => setLanguage("en")}
                className={`rounded-md px-2.5 py-1 text-xs font-semibold transition ${
                  language === "en"
                    ? "bg-cyan-500 text-zinc-950"
                    : "border border-cyan-300/20 bg-zinc-900/70 text-zinc-300 hover:text-cyan-100"
                }`}
              >
                EN
              </button>
              <button
                type="button"
                onClick={() => setLanguage("fr")}
                className={`rounded-md px-2.5 py-1 text-xs font-semibold transition ${
                  language === "fr"
                    ? "bg-cyan-500 text-zinc-950"
                    : "border border-cyan-300/20 bg-zinc-900/70 text-zinc-300 hover:text-cyan-100"
                }`}
              >
                FR
              </button>
            </div>
            <div className="grid items-center gap-8 md:grid-cols-[1.2fr_1fr]">
              <div>
                <p className="mb-3 text-xs uppercase tracking-[0.35em] text-cyan-200/80">
                  {t("realtimeTeamChat")}
                </p>
                <h1 className="mb-4 text-4xl font-black leading-tight text-zinc-100 md:text-6xl">
                  {t("orbitOtterChat")}
                </h1>
                <p className="mb-6 max-w-xl text-base text-zinc-300 md:text-lg">
                  {t("welcomeDescription")}
                </p>
                <p className="mb-8 text-sm text-cyan-200/90">
                  {t("welcomeTagline")}
                </p>
                <div className="flex flex-wrap items-center gap-3">
                  <button
                    onClick={() => {
                      setIsSignup(false);
                      setShowAuthForm(true);
                    }}
                    className="rounded-xl bg-cyan-500 px-6 py-3 font-semibold text-zinc-950 transition hover:bg-cyan-400"
                  >
                    {t("signIn")}
                  </button>
                  <button
                    onClick={() => {
                      setIsSignup(true);
                      setShowAuthForm(true);
                    }}
                    className="rounded-xl border border-cyan-300/30 bg-cyan-500/10 px-6 py-3 font-semibold text-cyan-100 transition hover:bg-cyan-500/20"
                  >
                    {t("createAccount")}
                  </button>
                </div>
              </div>
              <div className="relative mx-auto w-full max-w-[320px]">
                <div className="absolute inset-0 rounded-[2rem] bg-cyan-400/20 blur-2xl" />
                <div className="relative rounded-[2rem] border border-cyan-200/30 bg-zinc-900/70 p-5">
                  <PixelOtterMascot className="h-auto w-full" />
                </div>
              </div>
            </div>
          </div>
        </div>
      );
    }

    return (
      <div className="relative flex min-h-screen items-center justify-center overflow-hidden bg-[#060a1b] px-4 py-10 text-zinc-100">
        <div className="absolute inset-0 bg-[radial-gradient(circle_at_12%_12%,rgba(34,211,238,0.24),transparent_35%),radial-gradient(circle_at_86%_0%,rgba(37,99,235,0.24),transparent_34%),radial-gradient(circle_at_70%_85%,rgba(20,184,166,0.18),transparent_30%)]" />
        <div className="relative w-full max-w-4xl overflow-hidden rounded-[2rem] border border-cyan-300/20 bg-zinc-900/70 backdrop-blur-xl">
          <div className="flex items-center justify-end gap-2 border-b border-cyan-300/10 px-6 py-3">
            <span className="text-xs uppercase tracking-[0.2em] text-zinc-400">{t("language")}</span>
            <button
              type="button"
              onClick={() => setLanguage("en")}
              className={`rounded-md px-2.5 py-1 text-xs font-semibold transition ${
                language === "en"
                  ? "bg-cyan-500 text-zinc-950"
                  : "border border-cyan-300/20 bg-zinc-900/70 text-zinc-300 hover:text-cyan-100"
              }`}
            >
              EN
            </button>
            <button
              type="button"
              onClick={() => setLanguage("fr")}
              className={`rounded-md px-2.5 py-1 text-xs font-semibold transition ${
                language === "fr"
                  ? "bg-cyan-500 text-zinc-950"
                  : "border border-cyan-300/20 bg-zinc-900/70 text-zinc-300 hover:text-cyan-100"
              }`}
            >
              FR
            </button>
          </div>
          <div className="grid md:grid-cols-[1fr_1.1fr]">
            <div className="hidden border-r border-cyan-300/10 p-8 md:block">
              <PixelOtterMascot className="mx-auto mb-6 h-auto w-full max-w-[220px]" />
              <p className="text-xs uppercase tracking-[0.28em] text-cyan-200/80">{t("orbitOtterChat")}</p>
              <h2 className="mt-2 text-2xl font-black text-zinc-100">
                {isSignup ? t("joinTheOrbit") : t("welcomeBack")}
              </h2>
              <p className="mt-3 text-sm text-zinc-300">
                {isSignup
                  ? t("authLeftSignupDescription")
                  : t("authLeftSigninDescription")}
              </p>
            </div>
            <form onSubmit={handleAuth} className="p-6 md:p-8">
              <div className="mb-6 inline-flex rounded-xl border border-cyan-300/20 bg-zinc-900/60 p-1">
                <button
                  type="button"
                  onClick={() => setIsSignup(false)}
                  className={`rounded-lg px-4 py-2 text-sm font-semibold transition ${
                    !isSignup
                      ? "bg-cyan-500 text-zinc-950"
                      : "text-zinc-300 hover:text-cyan-100"
                  }`}
                >
                  {t("signIn")}
                </button>
                <button
                  type="button"
                  onClick={() => setIsSignup(true)}
                  className={`rounded-lg px-4 py-2 text-sm font-semibold transition ${
                    isSignup
                      ? "bg-cyan-500 text-zinc-950"
                      : "text-zinc-300 hover:text-cyan-100"
                  }`}
                >
                  {t("createAccount")}
                </button>
              </div>
              <h1 className="mb-1 text-3xl font-black">
                {isSignup ? t("authSignupTitle") : t("authSigninTitle")}
              </h1>
              <p className="mb-6 text-sm text-zinc-400">
                {isSignup
                  ? t("authSignupDescription")
                  : t("authSigninDescription")}
              </p>
              <div className="mb-3">
                <label className="mb-1 block text-sm text-zinc-300">{t("email")}</label>
                <input
                  type="email"
                  className="w-full rounded-xl border border-zinc-700 bg-zinc-800/80 p-3 outline-none transition focus:border-cyan-300/60 focus:ring-2 focus:ring-cyan-500/30"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  required
                />
              </div>
              {isSignup && (
                <div className="mb-3">
                  <label className="mb-1 block text-sm text-zinc-300">{t("username")}</label>
                  <input
                    type="text"
                    className="w-full rounded-xl border border-zinc-700 bg-zinc-800/80 p-3 outline-none transition focus:border-cyan-300/60 focus:ring-2 focus:ring-cyan-500/30"
                    value={username}
                    onChange={(e) => setUsername(e.target.value)}
                    required
                  />
                </div>
              )}
              <div className="mb-4">
                <label className="mb-1 block text-sm text-zinc-300">{t("password")}</label>
                <input
                  type="password"
                  className="w-full rounded-xl border border-zinc-700 bg-zinc-800/80 p-3 outline-none transition focus:border-cyan-300/60 focus:ring-2 focus:ring-cyan-500/30"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  required
                />
              </div>
              {authError && (
                <p className="mb-3 rounded-xl border border-rose-500/30 bg-rose-500/10 px-3 py-2 text-sm text-rose-200">
                  {authError}
                </p>
              )}
              <button
                type="submit"
                disabled={authLoading}
                className="w-full rounded-xl bg-cyan-500 py-3 font-semibold text-zinc-950 transition hover:bg-cyan-400 disabled:opacity-50"
              >
                {authLoading ? t("loading") : isSignup ? t("createAccount") : t("enterChat")}
              </button>
              <button
                type="button"
                onClick={() => {
                  setAuthError("");
                  setShowAuthForm(false);
                }}
                className="mt-3 w-full rounded-xl border border-zinc-700 bg-zinc-900/70 py-2.5 text-sm text-zinc-300 transition hover:border-cyan-300/30 hover:text-cyan-100"
              >
                {t("backToWelcomeScreen")}
              </button>
            </form>
          </div>
        </div>
      </div>
    );
  }

  return (
    <>
      <div className="pointer-events-none fixed right-4 top-4 z-[100] space-y-2" aria-live="polite">
        {toasts.map((toast) => (
          <div
            key={toast.id}
            className={`pointer-events-auto rounded-lg border px-3 py-2 text-sm shadow-lg ${
              toast.type === "error"
                ? "border-red-500/40 bg-red-950/90 text-red-100"
                : toast.type === "success"
                  ? "border-emerald-500/40 bg-emerald-950/90 text-emerald-100"
                  : "border-zinc-500/40 bg-zinc-900/95 text-zinc-100"
            }`}
            role="status"
          >
            {toast.text}
          </div>
        ))}
      </div>

      <div className="flex h-screen bg-zinc-900 text-zinc-100">
        <div className="w-20 border-r border-zinc-800 bg-zinc-950 p-3">
          <button
            onClick={openCreateServerModal}
            className="mb-4 flex h-12 w-12 items-center justify-center rounded-2xl bg-gradient-to-br from-cyan-500 to-blue-600 text-2xl font-bold text-white shadow-lg shadow-cyan-900/30 transition hover:scale-[1.03] hover:from-cyan-400 hover:to-blue-500"
            title={t("createServer")}
          >
            +
          </button>
          <button onClick={() => setIsSettingsOpen(true)} disabled={!selectedServerId} className="mb-4 flex h-12 w-12 items-center justify-center rounded-2xl border border-zinc-700 bg-zinc-800/80 text-lg transition hover:bg-zinc-700 disabled:opacity-40" title={selectedServerId ? t("openSettings") : t("selectServerFirst")}>⚙</button>
          <button onClick={() => { setProfileError(""); setProfileSuccess(""); setIsProfileSettingsOpen(true); }} className="mb-4 flex h-12 w-12 items-center justify-center rounded-2xl border border-zinc-700 bg-zinc-800/80 text-lg transition hover:bg-zinc-700" title={t("openProfileSettings")}>👤</button>
          <div className="space-y-2">
            {servers.map((server) => (
              <button key={server.id} onClick={() => setSelectedServerId(server.id)} className={`h-12 w-12 rounded-xl text-sm font-bold ${selectedServerId === server.id ? "bg-blue-600" : "bg-zinc-700 hover:bg-blue-500"}`}>
                {server.name?.slice(0, 2).toUpperCase() || "SR"}
              </button>
            ))}
          </div>
        </div>

        <div className="w-80 border-r border-zinc-800 bg-zinc-900">
          <div className="flex items-center justify-between border-b border-zinc-800 p-4 font-semibold">
            <span>{t("channels")}</span>
            <button onClick={logout} className={BTN_DANGER}>{t("logout")}</button>
          </div>
          <form onSubmit={joinServerByInvite} className="border-b border-zinc-800 p-3">
            <input type="text" value={inviteCodeInput} onChange={(e) => setInviteCodeInput(e.target.value)} placeholder={t("inviteCode")} className="w-full rounded bg-zinc-800 px-2 py-1 text-sm" disabled={joinLoading} />
          </form>
          <div className="grid grid-cols-2 gap-2 border-b border-zinc-800 p-3">
            <button
              onClick={handleCreateInviteCode}
              disabled={!selectedServerId || createInviteLoading}
              className="rounded-lg border border-cyan-400/30 bg-cyan-500/10 px-2 py-1 text-xs font-medium text-cyan-200 transition hover:bg-cyan-500/20 disabled:opacity-50"
            >
              {createInviteLoading ? t("creating") : t("createInvite")}
            </button>
            <button
              onClick={handleLeaveCurrentServer}
              disabled={!selectedServerId || isOwnerInSelectedServer}
              className="rounded-lg border border-rose-400/30 bg-rose-500/10 px-2 py-1 text-xs font-medium text-rose-200 transition hover:bg-rose-500/20 disabled:opacity-50"
              title={isOwnerInSelectedServer ? t("ownerCannotLeave") : undefined}
            >
              {t("leaveServer")}
            </button>
          </div>
          {lastInviteCode && (
            <div className="border-b border-zinc-800 px-3 py-2 text-xs text-emerald-300">{t("inviteLabel")}: {lastInviteCode}</div>
          )}
          <form onSubmit={handleCreateChannel} className="border-b border-zinc-800 p-3">
            <div className="mb-2 text-xs text-zinc-400">{t("createChannel")}</div>
            <div className="flex gap-2">
              <input type="text" value={newChannelName} onChange={(e) => setNewChannelName(e.target.value)} placeholder={t("channelTitle")} className="w-full rounded bg-zinc-800 px-2 py-1 text-sm" disabled={!selectedServerId} />
              <button type="submit" disabled={!selectedServerId || !newChannelName.trim() || createChannelLoading} className={BTN_PRIMARY}>{createChannelLoading ? t("adding") : t("add")}</button>
            </div>
            <input type="text" value={newChannelDescription} onChange={(e) => setNewChannelDescription(e.target.value)} placeholder={t("channelDescriptionOptional")} className="mt-2 w-full rounded bg-zinc-800 px-2 py-1 text-sm" disabled={!selectedServerId || createChannelLoading} />
            {!selectedServerId && <p className="mt-2 text-xs text-zinc-500">{t("selectServerFirst")}</p>}
          </form>
          {canAdminChannels && selectedChannelId && selectedChannel?.server_id && (
            <div className="border-b border-zinc-800 p-3">
              <div className="mb-2 text-xs text-zinc-400">{t("editSelectedChannel")}</div>
              <form onSubmit={handleUpdateChannel} className="space-y-2">
                <input type="text" value={channelEditName} onChange={(e) => setChannelEditName(e.target.value)} placeholder={t("channelTitle")} className="w-full rounded bg-zinc-800 px-2 py-1 text-sm" />
                <input type="text" value={channelEditTopic} onChange={(e) => setChannelEditTopic(e.target.value)} placeholder={t("channelDescriptionOptional")} className="w-full rounded bg-zinc-800 px-2 py-1 text-sm" />
                <input type="number" min={0} value={channelEditPosition} onChange={(e) => setChannelEditPosition(e.target.value)} placeholder={t("position")} className="w-full rounded bg-zinc-800 px-2 py-1 text-sm" />
                <button type="submit" disabled={updateChannelLoading} className={`w-full ${BTN_PRIMARY}`}>{updateChannelLoading ? t("saving") : t("saveChannelChanges")}</button>
              </form>
              <button onClick={confirmChannelDelete} className={`mt-2 w-full ${BTN_DANGER}`}>{t("deleteSelectedChannel")}</button>
            </div>
          )}
          <div className="border-b border-zinc-800 p-3 text-xs text-zinc-400">
            <div>{t("onlineNow")}: {onlineUsers.length}</div>
            <div>{t("membersTotal")}: {members.length}</div>
            <div>{t("status")}: {presenceLabel(myPresence)}</div>
            <select value={myPresence} onChange={(e) => handleSetPresence(e.target.value as Presence)} className="mt-2 w-full rounded bg-zinc-800 px-2 py-1">
              <option value="online">{t("online")}</option>
              <option value="away">{t("away")}</option>
              <option value="invisible">{t("invisible")}</option>
            </select>
          </div>
          <div className="h-[calc(100%-250px)] overflow-y-auto p-2">
            <div className="px-2 pb-1 text-[11px] uppercase tracking-[0.18em] text-zinc-500">{t("serverChannels")}</div>
            {channels.map((channel) => (
              <button key={channel.id} onClick={() => setSelectedChannelId(channel.id)} className={`mb-1 w-full rounded px-2 py-1 text-left text-sm ${selectedChannelId === channel.id ? "bg-zinc-700 text-white" : "text-zinc-400 hover:bg-zinc-800"}`}>
                <div className="flex items-center justify-between gap-2">
                  <span className="truncate">{getChannelLabel(channel)}</span>
                  <div className="flex items-center gap-1">
                    {(channelMentionCount[channel.id] ?? 0) > 0 && (
                      <span className="rounded-full border border-rose-400/40 bg-rose-500/20 px-1.5 py-0.5 text-[10px] font-semibold text-rose-200">
                        @{channelMentionCount[channel.id]}
                      </span>
                    )}
                    {(channelUnreadCount[channel.id] ?? 0) > 0 && (
                      <span className="rounded-full border border-cyan-400/40 bg-cyan-500/20 px-1.5 py-0.5 text-[10px] font-semibold text-cyan-100">
                        {channelUnreadCount[channel.id]}
                      </span>
                    )}
                  </div>
                </div>
              </button>
            ))}
            {channels.length === 0 && <p className="px-2 text-xs text-zinc-500">{t("noChannelsYet")}</p>}
            <div className="mt-4 px-2 pb-1 text-[11px] uppercase tracking-[0.18em] text-zinc-500">{t("directMessages")}</div>
            {dmChannels.map((channel) => (
              <button key={channel.id} onClick={() => setSelectedChannelId(channel.id)} className={`mb-1 w-full rounded px-2 py-1 text-left text-sm ${selectedChannelId === channel.id ? "bg-zinc-700 text-white" : "text-zinc-400 hover:bg-zinc-800"}`}>
                {getChannelLabel(channel)}
              </button>
            ))}
            {dmChannels.length === 0 && <p className="px-2 text-xs text-zinc-500">{t("noDirectMessagesYet")}</p>}
          </div>
        </div>

        <div className="flex-1 bg-zinc-800">
          <div className="flex h-14 items-center justify-between gap-3 border-b border-zinc-900 px-4">
            <div className="min-w-0 font-semibold">
              {selectedChannelId && selectedChannel
                ? getChannelLabel(selectedChannel)
                : t("noChannelSelected")}
              {selectedChannel?.topic ? (
                <span className="ml-3 text-xs font-normal text-zinc-400">{selectedChannel.topic}</span>
              ) : null}
            </div>
            {selectedChannel?.server_id === null && selectedDmPeerId && (
              <div className="flex items-center gap-2">
                {blockedUserIds.has(selectedDmPeerId) ? (
                  <button
                    type="button"
                    onClick={() => handleUnblockUser(selectedDmPeerId)}
                    className={BTN_MUTED}
                  >
                    {t("unblockUser")}
                  </button>
                ) : (
                  <button
                    type="button"
                    onClick={() => handleBlockUser(selectedDmPeerId)}
                    className={BTN_DANGER}
                  >
                    {t("blockUser")}
                  </button>
                )}
                <button
                  type="button"
                  onClick={handleDeleteSelectedDmHistory}
                  disabled={deleteDmLoading}
                  className={BTN_MUTED}
                >
                  {deleteDmLoading ? t("deletingConversation") : t("deleteDmHistory")}
                </button>
              </div>
            )}
          </div>

          <div className="h-[calc(100%-190px)] overflow-y-auto p-4">
            {!selectedChannelId && (
              <div className="rounded-xl border border-zinc-700 bg-zinc-900/70 p-4 text-sm text-zinc-300">
                <p className="font-semibold text-zinc-100">You need a channel to chat.</p>
                {canAdminChannels ? (
                  <p className="mt-1">{t("createOneFromLeft")}</p>
                ) : (
                  <p className="mt-1">{t("askAdminCreateChannel")}</p>
                )}
              </div>
            )}
            {selectedChannelId && (
              <div className="mb-4 space-y-3">
                <input
                  type="search"
                  value={messageSearchQuery}
                  onChange={(e) => setMessageSearchQuery(e.target.value)}
                  placeholder={t("searchMessages")}
                  className="w-full rounded border border-zinc-700 bg-zinc-800 px-3 py-2 text-sm text-zinc-100 outline-none focus:border-cyan-500"
                />
                {pinnedMessages.length > 0 && (
                  <div className="rounded border border-amber-500/30 bg-amber-500/10 p-3">
                    <div className="mb-2 text-xs font-semibold uppercase tracking-[0.16em] text-amber-200">
                      {t("pinnedMessages")}
                    </div>
                    <div className="space-y-2">
                      {pinnedMessages.map((message) => {
                        const author = getKnownMember(message.author_id);
                        const name =
                          memberDisplayName(author) ||
                          (message.author_id === currentUserId
                            ? memberDisplayName(currentMember)
                            : null) ||
                          shortUser(message.author_id);
                        return (
                          <button
                            key={`pinned-${message.id}`}
                            type="button"
                            onClick={() => setMessageSearchQuery(message.content)}
                            className="block w-full rounded bg-zinc-900/70 px-3 py-2 text-left text-xs text-zinc-200 hover:bg-zinc-800"
                          >
                            <span className="mr-2 font-semibold text-amber-200">{name}</span>
                            <span className="text-zinc-300">
                              {message.content.startsWith("gif::") ? "GIF" : message.content}
                            </span>
                          </button>
                        );
                      })}
                    </div>
                  </div>
                )}
              </div>
            )}
            {filteredMessages.length === 0 && messages.length > 0 && (
              <div className="rounded border border-dashed border-zinc-700 p-6 text-center text-sm text-zinc-500">
                {t("noSearchResults")}
              </div>
            )}
            {filteredMessages.map((message) => {
              const ownMessage = message.author_id === currentUserId;
              const author = getKnownMember(message.author_id);
              const resolvedAuthorName =
                memberDisplayName(author) ||
                (ownMessage ? memberDisplayName(currentMember) : null) ||
                shortUser(message.author_id);
              const displayName = ownMessage
                ? `${resolvedAuthorName} (${t("you")})`
                : resolvedAuthorName;
              const avatarUrl = resolveAvatarUrl(API_URL, author?.avatar_url);
              return (
                <div key={message.id} className="mb-3">
                  <div className="flex items-center gap-2 text-xs font-semibold text-blue-400">
                    {ownMessage ? (
                      <>
                        {avatarUrl ? (
                          <div aria-label="Author avatar" className="h-6 w-6 rounded-full border border-zinc-700 bg-cover bg-center" style={{ backgroundImage: `url("${avatarUrl}")` }} />
                        ) : (
                          <div className="flex h-6 w-6 items-center justify-center rounded-full border border-zinc-700 bg-zinc-700 text-[10px]">
                            {displayName.slice(0, 1).toUpperCase()}
                          </div>
                        )}
                        {displayName}
                      </>
                    ) : (
                      <button
                        type="button"
                        onClick={() =>
                          handleOpenDirectMessage(message.author_id, displayName, author)
                        }
                        className="flex items-center gap-2 rounded px-1 py-0.5 text-blue-300 transition hover:bg-zinc-700/60 hover:text-blue-200"
                        title={`${t("directMessages")}: ${displayName}`}
                      >
                        {avatarUrl ? (
                          <div aria-label="Author avatar" className="h-6 w-6 rounded-full border border-zinc-700 bg-cover bg-center" style={{ backgroundImage: `url("${avatarUrl}")` }} />
                        ) : (
                          <div className="flex h-6 w-6 items-center justify-center rounded-full border border-zinc-700 bg-zinc-700 text-[10px]">
                            {displayName.slice(0, 1).toUpperCase()}
                          </div>
                        )}
                        {displayName}
                      </button>
                    )}
                  </div>
                  {editingMessageId === message.id ? (
                    <div className="mt-1">
                      <input type="text" value={editingMessageContent} onChange={(e) => setEditingMessageContent(e.target.value)} className="w-full rounded bg-zinc-700 px-3 py-2 text-sm" />
                      <div className="mt-2 flex gap-2">
                        <button onClick={handleUpdateMessage} className={BTN_PRIMARY}>{t("save")}</button>
                        <button onClick={() => { setEditingMessageId(null); setEditingMessageContent(""); }} className={BTN_MUTED}>{t("cancel")}</button>
                      </div>
                    </div>
	                  ) : (
	                    <div className="mt-1 inline-block rounded bg-zinc-700/70 px-3 py-2 text-sm">
	                      {message.pinned_at && (
	                        <span className="mr-2 rounded bg-amber-400/20 px-1.5 py-0.5 text-[10px] font-semibold uppercase text-amber-200">
	                          {t("pinned")}
	                        </span>
	                      )}
	                      {renderMessageContent(message.content, API_URL)}
	                    </div>
	                  )}
	                  <div className="mt-1 flex gap-3 text-xs">
	                    {ownMessage && editingMessageId !== message.id && (
	                      <button onClick={() => beginEditMessage(message)} className="text-zinc-400 hover:text-emerald-300">{t("edit")}</button>
	                    )}
	                    {canPinMessage(message) && (
	                      <button onClick={() => handleTogglePinMessage(message)} className="text-zinc-400 hover:text-amber-300">
	                        {message.pinned_at ? t("unpin") : t("pin")}
	                      </button>
	                    )}
	                    {canDeleteMessage(message) && (
                      <button onClick={() => requestDeleteMessage(message.id)} className="text-zinc-400 hover:text-red-400">
                        {ownMessage ? t("delete") : t("deleteModerate")}
                      </button>
                    )}
                  </div>
                  <div className="mt-2 flex flex-wrap items-center gap-2">
                    {(reactionsByMessage[message.id] ?? []).map((reaction) => {
                      const count = reaction.count ?? 0;
                      const reacted = reaction.reacted ?? false;
                      return (
                        <button
                          key={`${message.id}-${reaction.emoji}`}
                          onClick={() => handleToggleReaction(message.id, reaction.emoji)}
                          className={`rounded-full border px-2 py-0.5 text-xs ${
                            reacted
                              ? "border-blue-400 bg-blue-900/40 text-blue-100"
                              : "border-zinc-600 bg-zinc-800 text-zinc-300 hover:border-zinc-400"
                          }`}
                        >
                          {reaction.emoji} {count}
                        </button>
                      );
                    })}
                    <button
                      type="button"
                      onClick={() =>
                        setReactionPickerByMessage((prev) => ({
                          ...prev,
                          [message.id]: !prev[message.id],
                        }))
                      }
                      className="rounded-full border border-cyan-400/40 bg-cyan-500/10 px-2 py-0.5 text-xs text-cyan-200 hover:bg-cyan-500/20"
                      title={t("addReaction")}
                    >
                      +
                    </button>
                    {reactionPickerByMessage[message.id] && (
                      <div className="flex flex-wrap items-center gap-1 rounded-lg border border-zinc-600 bg-zinc-900/95 px-2 py-1">
                        {REACTION_OPTIONS.map((emoji) => (
                          <button
                            key={`${message.id}-extra-${emoji}`}
                            type="button"
                            onClick={() => {
                              handleToggleReaction(message.id, emoji);
                              setReactionPickerByMessage((prev) => ({
                                ...prev,
                                [message.id]: false,
                              }));
                            }}
                            className="rounded px-1.5 py-0.5 text-sm hover:bg-zinc-700"
                          >
                            {emoji}
                          </button>
                        ))}
                      </div>
                    )}
                  </div>
                </div>
              );
            })}
          </div>

          <div className="border-t border-zinc-900 px-4 py-2 text-xs text-zinc-400">
            {typingText}
          </div>

          <div className="border-t border-zinc-900 px-4 py-2">
            <div className="mb-2 flex gap-2">
              <button onClick={() => insertEmoji("😀")} disabled={!selectedChannelId} className={BTN_MUTED}>😀</button>
              <button onClick={() => insertEmoji("🔥")} disabled={!selectedChannelId} className={BTN_MUTED}>🔥</button>
              <button onClick={() => insertEmoji("✅")} disabled={!selectedChannelId} className={BTN_MUTED}>✅</button>

              <button
                onClick={() => setShowGifPicker((v) => !v)}
                disabled={!selectedChannelId}
                className={`rounded px-2 py-1 text-xs font-semibold transition-colors disabled:opacity-40 ${
                  showGifPicker
                    ? "bg-violet-600 text-white"
                    : "bg-zinc-700 text-zinc-300 hover:bg-zinc-600"
                }`}
              >
                🎬 GIF
              </button>
              <input
                ref={messageFileInputRef}
                type="file"
                className="hidden"
                onChange={(e) => handleMessageFileUpload(e.target.files?.[0] ?? null)}
              />
              <button
                type="button"
                onClick={() => messageFileInputRef.current?.click()}
                disabled={!selectedChannelId || fileUploadLoading || isSelectedDmBlockedByMe}
                className="rounded px-2 py-1 text-xs font-semibold text-zinc-300 transition-colors hover:bg-zinc-600 disabled:opacity-40"
                title={t("attachFile")}
              >
                {fileUploadLoading ? t("uploadingFile") : "📎"}
              </button>
            </div>

            <div className="relative">
              {showGifPicker && (
                <GifPicker
                  onSelect={handleGifSelect}
                  onClose={() => setShowGifPicker(false)}
                />
              )}
              <form onSubmit={handleSendMessage}>
                <input
                  type="text"
                  value={newMessage}
                  onChange={(e) => {
                    setNewMessage(e.target.value);
                    handleSendTyping(e.target.value ? "typing_start" : "typing_stop");
                  }}
                  onBlur={() => handleSendTyping("typing_stop")}
                  placeholder={
                    isSelectedDmBlockedByMe
                      ? t("dmBlockedByPolicy")
                      : selectedChannelId
                        ? t("writeMessage")
                        : t("selectChannelToWrite")
                  }
                  className="w-full rounded-lg bg-zinc-700 px-4 py-3 text-white outline-none focus:ring-2 focus:ring-blue-500"
                  disabled={!selectedChannelId || chatLoading || isSelectedDmBlockedByMe}
                />
              </form>
              {mentionSuggestions.length > 0 && (
                <div className="absolute bottom-[calc(100%+0.5rem)] left-0 right-0 z-40 overflow-hidden rounded-lg border border-zinc-600 bg-zinc-900/95 shadow-xl">
                  {mentionSuggestions.map((item) => (
                    <button
                      key={item.userId}
                      type="button"
                      onClick={() => insertMention(item.handle)}
                      className="flex w-full items-center justify-between px-3 py-2 text-left text-sm text-zinc-200 transition hover:bg-zinc-800"
                    >
                      <span className="truncate">{item.display}</span>
                      <span className="ml-3 shrink-0 text-xs text-cyan-300">@{item.handle}</span>
                    </button>
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>
      </div>

      {isSettingsOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4">
          <div role="dialog" aria-modal="true" aria-labelledby="server-settings-title" className="h-[85vh] w-full max-w-6xl overflow-hidden rounded-2xl border border-zinc-700 bg-zinc-900 text-zinc-100">
            <div className="flex items-center justify-between border-b border-zinc-800 px-5 py-4">
              <div>
                <h2 id="server-settings-title" className="text-lg font-semibold">{t("serverSettings")}</h2>
                <p className="text-xs text-zinc-400">{t("serverSettingsSubtitle")}</p>
              </div>
              <button ref={settingsCloseBtnRef} onClick={() => setIsSettingsOpen(false)} className={BTN_MUTED}>{t("close")}</button>
            </div>
            <div className="grid h-[calc(85vh-74px)] gap-4 overflow-y-auto p-4 lg:grid-cols-[360px_1fr]">
              <section className="space-y-3">
                <div className="rounded bg-zinc-800 p-3">
                  <div className="mb-2 text-xs text-zinc-300">{t("languageInSettings")}</div>
                  <div className="grid grid-cols-2 gap-2">
                    <button type="button" onClick={() => setLanguage("en")} className={language === "en" ? BTN_PRIMARY : BTN_MUTED}>
                      {t("english")}
                    </button>
                    <button type="button" onClick={() => setLanguage("fr")} className={language === "fr" ? BTN_PRIMARY : BTN_MUTED}>
                      {t("french")}
                    </button>
                  </div>
                </div>
                <div className="rounded bg-zinc-800 p-3">
                  <div className="mb-2 text-xs text-zinc-300">{t("presence")}</div>
                  <div className="text-xs text-zinc-400">{t("onlineNow")}: {onlineUsers.length}</div>
                  <div className="text-xs text-zinc-400">{t("membersTotal")}: {members.length}</div>
                  <div className="mt-2 text-xs text-zinc-400">{t("currentStatus")}: {presenceLabel(myPresence)}</div>
                </div>
                {canManageMembers && (
                  <div className="rounded bg-zinc-800 p-3">
                    <div className="mb-2 text-xs font-semibold text-zinc-200">{t("banOptions")}</div>
                    <input
                      type="text"
                      value={banReason}
                      onChange={(e) => setBanReason(e.target.value)}
                      placeholder={t("banReasonOptional")}
                      className="mb-2 w-full rounded border border-zinc-600 bg-zinc-700 px-2 py-1.5 text-sm outline-none focus:border-cyan-400/60"
                    />
                    <input
                      type="number"
                      min={1}
                      value={banDurationHours}
                      onChange={(e) => setBanDurationHours(e.target.value)}
                      placeholder={t("banDurationHint")}
                      className="w-full rounded border border-zinc-600 bg-zinc-700 px-2 py-1.5 text-sm outline-none focus:border-cyan-400/60"
                    />
                    <div className="mt-2 text-[11px] text-zinc-400">{t("banQuickDuration")}</div>
                    <div className="mt-1 flex flex-wrap gap-2">
                      <button type="button" onClick={() => setBanDurationHours("1")} className={BTN_MUTED}>
                        1h
                      </button>
                      <button type="button" onClick={() => setBanDurationHours("24")} className={BTN_MUTED}>
                        24h
                      </button>
                      <button type="button" onClick={() => setBanDurationHours("168")} className={BTN_MUTED}>
                        7d
                      </button>
                      <button type="button" onClick={() => setBanDurationHours("")} className={BTN_MUTED}>
                        {t("clearDuration")}
                      </button>
                    </div>
                    <div className="mt-2 text-[11px] text-zinc-500">
                      {t("currentDuration")}: {banDurationHours.trim() ? `${banDurationHours}h` : t("permanent")}
                    </div>
                  </div>
                )}
                {canManageMembers && (
                  <div className="rounded bg-zinc-800 p-3">
                    <div className="mb-2 text-xs text-zinc-300">{t("bannedUsers")}</div>
                    <div className="space-y-2">
                      {bans.map((ban) => (
                        <div key={`${ban.server_id}-${ban.user_id}`} className="rounded bg-zinc-700 p-2 text-xs">
                          {(() => {
                            const cached = getKnownMember(ban.user_id);
                            const banName =
                              ban.nickname ||
                              ban.username ||
                              memberDisplayName(cached) ||
                              shortUser(ban.user_id);
                            const banAvatar = resolveAvatarUrl(
                              API_URL,
                              ban.avatar_url ?? cached?.avatar_url ?? null,
                            );
                            return (
                              <div className="mb-1 flex items-center gap-2">
                                {banAvatar ? (
                                  <div aria-label="Banned user avatar" className="h-6 w-6 rounded-full border border-zinc-600 bg-cover bg-center" style={{ backgroundImage: `url("${banAvatar}")` }} />
                                ) : (
                                  <div className="flex h-6 w-6 items-center justify-center rounded-full border border-zinc-600 bg-zinc-600/60 text-[10px] font-semibold">
                                    {banName.slice(0, 1).toUpperCase()}
                                  </div>
                                )}
                                <div className="font-medium">{banName}</div>
                              </div>
                            );
                          })()}
                          <div className="text-zinc-400">{ban.reason || t("noReason")}</div>
                          <div className="text-zinc-500">{ban.expires_at ? `${t("until")} ${ban.expires_at}` : t("permanent")}</div>
                          <button onClick={() => handleUnban(ban.user_id)} className={`mt-2 ${BTN_PRIMARY}`}>{t("unban")}</button>
                        </div>
                      ))}
                      {bans.length === 0 && <div className="text-xs text-zinc-500">{t("noBannedUsers")}</div>}
                    </div>
                  </div>
                )}
              </section>
              <section className="space-y-3">
                <div className="rounded bg-zinc-800 p-3 text-sm font-semibold">{t("members")}</div>
                {members.map((member) => {
                  const isOnline = onlineUsers.includes(member.user_id);
                  const isSelf = member.user_id === currentUserId;
                  const status = presenceMap[member.user_id] ?? (isOnline ? "online" : "invisible");
                  const canAct = canTarget(currentRole, member.role, isSelf);
                  const memberSnapshot = getKnownMember(member.user_id) ?? member;
                  const memberName = isSelf ? t("you") : memberDisplayName(memberSnapshot) || shortUser(member.user_id);
                  const avatarUrl = resolveAvatarUrl(API_URL, memberSnapshot.avatar_url);
                  const mentionHandle = getMentionHandleForMember(member);
                  return (
                    <div key={member.user_id} className="rounded bg-zinc-800 p-3">
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-2 text-sm">
                          {avatarUrl ? (
                            <div aria-label="Member avatar" className="h-7 w-7 rounded-full border border-zinc-700 bg-cover bg-center" style={{ backgroundImage: `url("${avatarUrl}")` }} />
                          ) : (
                            <div className="flex h-7 w-7 items-center justify-center rounded-full border border-zinc-700 bg-zinc-700 text-xs">{memberName.slice(0, 1).toUpperCase()}</div>
                          )}
                          {memberName}
                        </div>
                        <div className={`text-xs ${status === "online" ? "text-emerald-300" : status === "away" ? "text-amber-300" : "text-zinc-500"}`}>{status}</div>
                      </div>
                      <div className="mt-1 text-xs text-zinc-400">{t("role")}: {roleLabel(member.role)}</div>
                      <div className="mt-2 flex flex-wrap gap-2">
                        {!isSelf && (
                          <button onClick={() => mentionHandle && insertMention(mentionHandle)} disabled={!selectedChannelId || !mentionHandle} className={BTN_MUTED} title={!mentionHandle ? t("mention") : selectedChannelId ? `${t("mention")} @${mentionHandle}` : t("selectChannelToWrite")}>
                            {mentionHandle ? `@${mentionHandle}` : t("mention")}
                          </button>
                        )}
                        {!isSelf && (
                          <button
                            onClick={() =>
                              handleOpenDirectMessage(
                                member.user_id,
                                memberName,
                                memberSnapshot,
                              )
                            }
                            className={BTN_PRIMARY}
                          >
                            {t("dm")}
                          </button>
                        )}
                        {!isSelf && (
                          blockedUserIds.has(member.user_id) ? (
                            <button
                              onClick={() => handleUnblockUser(member.user_id)}
                              className={BTN_MUTED}
                            >
                              {t("unblockUser")}
                            </button>
                          ) : (
                            <button
                              onClick={() => handleBlockUser(member.user_id)}
                              className={BTN_DANGER}
                            >
                              {t("blockUser")}
                            </button>
                          )
                        )}
                        {canAct && canManageMembers && (
                          <>
                            <button onClick={() => handleKick(member.user_id)} className={BTN_DANGER}>{t("kick")}</button>
                            <button onClick={() => handleTemporaryBan(member.user_id)} className={BTN_MUTED}>{t("tempBan")}</button>
                            <button onClick={() => handlePermanentBan(member.user_id)} className={BTN_DANGER}>{t("permanentBan")}</button>
                          </>
                        )}
                      </div>
                      {canOwnerManage && !isSelf && normalizeRole(member.role) !== "OWNER" && (
                        <div className="mt-2 grid grid-cols-2 gap-2">
                          <button onClick={() => handleUpdateRole(member.user_id, "ADMIN")} className={BTN_PRIMARY}>{t("setAdmin")}</button>
                          <button onClick={() => handleUpdateRole(member.user_id, "MEMBER")} className={BTN_MUTED}>{t("setMember")}</button>
                          <button onClick={() => handleTransferOwnership(member.user_id)} className={`col-span-2 ${BTN_MUTED}`}>{t("transferOwnership")}</button>
                        </div>
                      )}
                    </div>
                  );
                })}
                {members.length === 0 && <div className="rounded bg-zinc-800 p-3 text-xs text-zinc-500">{t("noMembersLoaded")}</div>}
              </section>
            </div>
          </div>
        </div>
      )}

      {isProfileSettingsOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4">
          <div role="dialog" aria-modal="true" aria-labelledby="profile-settings-title" className="w-full max-w-2xl rounded-2xl border border-zinc-700 bg-zinc-900 p-5 text-zinc-100">
            <div className="mb-4 flex items-center justify-between">
              <div>
                <h2 id="profile-settings-title" className="text-lg font-semibold">{t("profileSettings")}</h2>
                <p className="text-xs text-zinc-400">{t("profileSettingsSubtitle")}</p>
              </div>
              <button ref={profileCloseBtnRef} onClick={() => setIsProfileSettingsOpen(false)} className={BTN_MUTED}>{t("close")}</button>
            </div>
            <form onSubmit={handleProfileUpdate} className="rounded bg-zinc-800 p-4">
              {profileError && <div className="mb-3 rounded border border-red-500/50 bg-red-950/40 px-3 py-2 text-xs text-red-300">{profileError}</div>}
              {profileSuccess && <div className="mb-3 rounded border border-emerald-500/50 bg-emerald-950/40 px-3 py-2 text-xs text-emerald-300">{profileSuccess}</div>}
              <div className="mb-2 text-xs text-zinc-300">{t("account")}</div>
              <div className="mb-3 flex items-center gap-3">
                {profilePreviewAvatar ? (
                  <div aria-label="Avatar preview" className="h-16 w-16 rounded-full border border-zinc-700 bg-cover bg-center" style={{ backgroundImage: `url("${profilePreviewAvatar}")` }} />
                ) : (
                  <div className="flex h-16 w-16 items-center justify-center rounded-full border border-zinc-700 bg-zinc-700 text-lg font-semibold">{(profileNickname || profileUsername || "U").slice(0, 1).toUpperCase()}</div>
                )}
                <div className="text-xs text-zinc-400">{t("addChangeAvatarHint")}</div>
              </div>
              <input type="email" value={profileEmail} onChange={(e) => setProfileEmail(e.target.value)} placeholder={t("email")} className="mb-2 w-full rounded bg-zinc-700 px-2 py-2 text-sm" />
              <input type="text" value={profileUsername} onChange={(e) => setProfileUsername(e.target.value)} placeholder={t("username")} className="mb-2 w-full rounded bg-zinc-700 px-2 py-2 text-sm" />
              <input type="text" value={profileNickname} onChange={(e) => setProfileNickname(e.target.value)} placeholder={t("nickname")} className="mb-2 w-full rounded bg-zinc-700 px-2 py-2 text-sm" />
              <select value={profileDisplayNameMode} onChange={(e) => setProfileDisplayNameMode(e.target.value as DisplayNameMode)} className="mb-2 w-full rounded bg-zinc-700 px-2 py-2 text-sm">
                <option value="nickname">{t("showNicknameInChat")}</option>
                <option value="username">{t("showUsernameInChat")}</option>
              </select>
              <input type="text" value={profileAvatarUrl} onChange={(e) => setProfileAvatarUrl(e.target.value)} placeholder={t("avatarUrl")} className="mb-3 w-full rounded bg-zinc-700 px-2 py-2 text-sm" />
              <div className="mb-3 rounded border border-zinc-700 bg-zinc-900 p-3">
                <div className="mb-2 text-xs text-zinc-300">{t("uploadAvatarFromComputer")}</div>
                <input type="file" accept="image/png,image/jpeg,image/webp,image/gif" onChange={(e) => setProfileAvatarFile(e.target.files?.[0] ?? null)} className="mb-2 w-full text-xs" />
                <button type="button" onClick={handleAvatarUpload} disabled={!profileAvatarFile || profileLoading} className={`w-full ${BTN_PRIMARY}`}>{t("uploadAvatar")}</button>
              </div>
              <div className="mb-3 text-xs text-zinc-500">{t("userId")}: {currentUserId ?? t("unknown")}</div>
              <button type="submit" disabled={profileLoading} className={`w-full ${BTN_PRIMARY}`}>{profileLoading ? t("saving") : t("saveProfile")}</button>
            </form>
          </div>
        </div>
      )}

      {isCreateServerModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
          <form role="dialog" aria-modal="true" aria-labelledby="create-server-title" onSubmit={submitCreateServer} className="w-full max-w-md rounded-xl bg-zinc-900 p-5">
            <h2 id="create-server-title" className="mb-3 text-lg font-semibold text-zinc-100">{t("createServer")}</h2>
            <input ref={createServerInputRef} type="text" value={newServerName} onChange={(e) => setNewServerName(e.target.value)} placeholder={t("serverName")} className="mb-4 w-full rounded bg-zinc-800 px-3 py-2 text-zinc-100" />
            <input type="text" value={newServerInitialChannelName} onChange={(e) => setNewServerInitialChannelName(e.target.value)} placeholder={t("firstChannelTitle")} className="mb-2 w-full rounded bg-zinc-800 px-3 py-2 text-zinc-100" />
            <input type="text" value={newServerInitialChannelDescription} onChange={(e) => setNewServerInitialChannelDescription(e.target.value)} placeholder={t("firstChannelDescription")} className="mb-4 w-full rounded bg-zinc-800 px-3 py-2 text-zinc-100" />
            <div className="flex justify-end gap-2">
              <button type="button" onClick={() => setIsCreateServerModalOpen(false)} className={BTN_MUTED}>{t("cancel")}</button>
              <button type="submit" disabled={createServerLoading || !newServerName.trim()} className={BTN_PRIMARY}>{createServerLoading ? t("creating") : t("create")}</button>
            </div>
          </form>
        </div>
      )}

      {confirmDeleteChannelOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
          <div role="dialog" aria-modal="true" aria-labelledby="delete-channel-title" className="w-full max-w-md rounded-xl bg-zinc-900 p-5 text-zinc-100">
            <h2 id="delete-channel-title" className="mb-2 text-lg font-semibold">{t("deleteChannelTitle")}</h2>
            <p className="mb-4 text-sm text-zinc-300">{t("cannotUndo")}</p>
            <div className="flex justify-end gap-2">
              <button ref={deleteChannelCancelRef} onClick={() => setConfirmDeleteChannelOpen(false)} className={BTN_MUTED}>{t("cancel")}</button>
              <button onClick={handleDeleteChannel} disabled={deleteChannelLoading} className={BTN_DANGER}>{deleteChannelLoading ? t("deleting") : t("delete")}</button>
            </div>
          </div>
        </div>
      )}

      {confirmDeleteMessageId && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
          <div role="dialog" aria-modal="true" aria-labelledby="delete-message-title" className="w-full max-w-md rounded-xl bg-zinc-900 p-5 text-zinc-100">
            <h2 id="delete-message-title" className="mb-2 text-lg font-semibold">{t("deleteMessageTitle")}</h2>
            <p className="mb-4 text-sm text-zinc-300">{t("cannotUndo")}</p>
            <div className="flex justify-end gap-2">
              <button ref={deleteMessageCancelRef} onClick={() => setConfirmDeleteMessageId(null)} className={BTN_MUTED}>{t("cancel")}</button>
              <button onClick={handleDeleteMessage} disabled={deleteMessageLoading} className={BTN_DANGER}>{deleteMessageLoading ? t("deleting") : t("delete")}</button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
