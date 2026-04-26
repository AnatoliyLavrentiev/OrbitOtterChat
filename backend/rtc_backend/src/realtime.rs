use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresenceStatus {
    Online,
    Away,
    Invisible,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum WsEvent {
    PresenceJoined {
        server_id: Uuid,
        user_id: Uuid,
        connected_users: Vec<Uuid>,
    },
    PresenceLeft {
        server_id: Uuid,
        user_id: Uuid,
        connected_users: Vec<Uuid>,
    },
    TypingStart {
        server_id: Uuid,
        channel_id: Uuid,
        user_id: Uuid,
    },
    TypingStop {
        server_id: Uuid,
        channel_id: Uuid,
        user_id: Uuid,
    },
    MessageNew {
        server_id: Option<Uuid>,
        channel_id: Uuid,
        message_id: Uuid,
        author_id: Uuid,
        content: String,
        dm_member_ids: Option<Vec<Uuid>>,
    },
    MessageDeleted {
        server_id: Option<Uuid>,
        channel_id: Uuid,
        message_id: Uuid,
        deleted_by: Uuid,
        dm_member_ids: Option<Vec<Uuid>>,
    },
    MessageReactionsUpdated {
        server_id: Option<Uuid>,
        channel_id: Uuid,
        message_id: Uuid,
        dm_member_ids: Option<Vec<Uuid>>,
    },
    StatusUpdated {
        server_id: Uuid,
        user_id: Uuid,
        status: PresenceStatus,
    },
    ServerBanApplied {
        server_id: Uuid,
        user_id: Uuid,
        banned_by: Uuid,
        reason: Option<String>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    },
}

#[derive(Clone)]
pub struct WsHub {
    tx: broadcast::Sender<WsEvent>,
    online: Arc<RwLock<HashMap<Uuid, HashSet<Uuid>>>>,
    statuses: Arc<RwLock<HashMap<Uuid, HashMap<Uuid, PresenceStatus>>>>,
}

impl WsHub {
    pub fn new(buffer: usize) -> Self {
        let (tx, _) = broadcast::channel(buffer);
        Self {
            tx,
            online: Arc::new(RwLock::new(HashMap::new())),
            statuses: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<WsEvent> {
        self.tx.subscribe()
    }

    pub fn publish(&self, event: WsEvent) {
        let _ = self.tx.send(event);
    }

    pub async fn user_join(&self, server_id: Uuid, user_id: Uuid) -> Vec<Uuid> {
        let mut online = self.online.write().await;
        let users = online.entry(server_id).or_default();
        users.insert(user_id);
        let mut statuses = self.statuses.write().await;
        statuses
            .entry(server_id)
            .or_default()
            .insert(user_id, PresenceStatus::Online);
        users.iter().copied().collect()
    }

    pub async fn user_leave(&self, server_id: Uuid, user_id: Uuid) -> Vec<Uuid> {
        let mut online = self.online.write().await;
        if let Some(users) = online.get_mut(&server_id) {
            users.remove(&user_id);
            if users.is_empty() {
                online.remove(&server_id);
                let mut statuses = self.statuses.write().await;
                statuses.remove(&server_id);
                return Vec::new();
            }
            let mut statuses = self.statuses.write().await;
            if let Some(s) = statuses.get_mut(&server_id) {
                s.remove(&user_id);
            }
            return users.iter().copied().collect();
        }
        Vec::new()
    }

    pub async fn set_status(&self, server_id: Uuid, user_id: Uuid, status: PresenceStatus) {
        let mut statuses = self.statuses.write().await;
        statuses
            .entry(server_id)
            .or_default()
            .insert(user_id, status);
    }
}
