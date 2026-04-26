use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use serde::Deserialize;
use uuid::Uuid;

use crate::errors::AppError;
use crate::realtime::{PresenceStatus, WsEvent};
use crate::{repositories, AppState};

#[derive(Debug, Deserialize)]
pub struct WsParams {
    pub token: String,
    pub server_id: Uuid,
    pub channel_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum ClientEvent {
    TypingStart { channel_id: Uuid },
    TypingStop { channel_id: Uuid },
    SetStatus { status: PresenceStatus },
}

pub async fn ws_handler(
    State(state): State<AppState>,
    Query(params): Query<WsParams>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    let user_id = state.jwt.verify_access_token(&params.token)?;
    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let is_member = repositories::server_members::is_member(&mut conn, params.server_id, user_id)?;
    if !is_member {
        return Err(AppError::Forbidden("not a member of this server".into()));
    }

    let hub = state.ws.clone();
    Ok(ws.on_upgrade(move |socket| {
        handle_socket(socket, hub, user_id, params.server_id, params.channel_id)
    }))
}

async fn handle_socket(
    mut socket: WebSocket,
    hub: crate::realtime::WsHub,
    user_id: Uuid,
    server_id: Uuid,
    channel_id: Option<Uuid>,
) {
    let mut rx = hub.subscribe();
    let connected_users = hub.user_join(server_id, user_id).await;
    hub.publish(WsEvent::PresenceJoined {
        server_id,
        user_id,
        connected_users,
    });
    hub.publish(WsEvent::StatusUpdated {
        server_id,
        user_id,
        status: PresenceStatus::Online,
    });

    loop {
        tokio::select! {
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(evt) = serde_json::from_str::<ClientEvent>(&text) {
                            match evt {
                                ClientEvent::TypingStart { channel_id: cid } => {
                                    if channel_id.is_none() || channel_id == Some(cid) {
                                        hub.publish(WsEvent::TypingStart {
                                            server_id,
                                            channel_id: cid,
                                            user_id,
                                        });
                                    }
                                }
                                ClientEvent::TypingStop { channel_id: cid } => {
                                    if channel_id.is_none() || channel_id == Some(cid) {
                                        hub.publish(WsEvent::TypingStop {
                                            server_id,
                                            channel_id: cid,
                                            user_id,
                                        });
                                    }
                                }
                                ClientEvent::SetStatus { status } => {
                                    hub.set_status(server_id, user_id, status.clone()).await;
                                    hub.publish(WsEvent::StatusUpdated {
                                        server_id,
                                        user_id,
                                        status,
                                    });
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
            event = rx.recv() => {
                let Ok(event) = event else { continue; };
                if !should_deliver(&event, server_id, channel_id, user_id) {
                    continue;
                }

                let Ok(payload) = serde_json::to_string(&event) else { continue; };
                if socket.send(Message::Text(payload.into())).await.is_err() {
                    break;
                }
            }
        }
    }

    let connected_users = hub.user_leave(server_id, user_id).await;
    hub.publish(WsEvent::PresenceLeft {
        server_id,
        user_id,
        connected_users,
    });
}

fn should_deliver(
    event: &WsEvent,
    server_id: Uuid,
    channel_id: Option<Uuid>,
    user_id: Uuid,
) -> bool {
    match event {
        WsEvent::PresenceJoined { server_id: sid, .. }
        | WsEvent::PresenceLeft { server_id: sid, .. } => *sid == server_id,

        WsEvent::StatusUpdated { server_id: sid, .. } => *sid == server_id,
        WsEvent::ServerBanApplied {
            server_id: sid,
            user_id: target_user_id,
            ..
        } => *sid == server_id && *target_user_id == user_id,

        WsEvent::TypingStart {
            server_id: sid,
            channel_id: cid,
            ..
        }
        | WsEvent::TypingStop {
            server_id: sid,
            channel_id: cid,
            ..
        } => *sid == server_id && channel_id.map(|c| c == *cid).unwrap_or(true),

        WsEvent::MessageNew {
            server_id: sid,
            channel_id: cid,
            dm_member_ids,
            ..
        }
        | WsEvent::MessageDeleted {
            server_id: sid,
            channel_id: cid,
            dm_member_ids,
            ..
        }
        | WsEvent::MessageReactionsUpdated {
            server_id: sid,
            channel_id: cid,
            dm_member_ids,
            ..
        } => match sid {
            Some(s) => *s == server_id && channel_id.map(|c| c == *cid).unwrap_or(true),
            None => dm_member_ids
                .as_ref()
                .map(|members| members.contains(&user_id))
                .unwrap_or(false),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_deliver_presence_inside_server() {
        let server_id = Uuid::new_v4();
        let other_server = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let event = WsEvent::PresenceJoined {
            server_id,
            user_id,
            connected_users: vec![user_id],
        };
        let other_event = WsEvent::PresenceLeft {
            server_id: other_server,
            user_id,
            connected_users: vec![],
        };

        assert!(should_deliver(&event, server_id, None, user_id));
        assert!(!should_deliver(&other_event, server_id, None, user_id));
    }

    #[test]
    fn should_deliver_status_inside_server() {
        let server_id = Uuid::new_v4();
        let other_server = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let event = WsEvent::StatusUpdated {
            server_id,
            user_id,
            status: PresenceStatus::Away,
        };
        let other_event = WsEvent::StatusUpdated {
            server_id: other_server,
            user_id,
            status: PresenceStatus::Online,
        };

        assert!(should_deliver(
            &event,
            server_id,
            Some(Uuid::new_v4()),
            user_id
        ));
        assert!(!should_deliver(
            &other_event,
            server_id,
            Some(Uuid::new_v4()),
            user_id
        ));
    }

    #[test]
    fn should_filter_channel_scoped_events_by_channel() {
        let server_id = Uuid::new_v4();
        let channel_id = Uuid::new_v4();
        let other_channel = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let typing = WsEvent::TypingStart {
            server_id,
            channel_id,
            user_id,
        };
        let message = WsEvent::MessageNew {
            server_id: Some(server_id),
            channel_id: other_channel,
            message_id: Uuid::new_v4(),
            author_id: user_id,
            content: "hello".into(),
            dm_member_ids: None,
        };

        assert!(should_deliver(
            &typing,
            server_id,
            Some(channel_id),
            user_id
        ));
        assert!(!should_deliver(
            &message,
            server_id,
            Some(channel_id),
            user_id
        ));
        assert!(should_deliver(&message, server_id, None, user_id));
    }

    #[test]
    fn should_deliver_dm_events_only_to_dm_members() {
        let server_id = Uuid::new_v4();
        let channel_id = Uuid::new_v4();
        let recipient = Uuid::new_v4();
        let outsider = Uuid::new_v4();

        let dm_event = WsEvent::MessageNew {
            server_id: None,
            channel_id,
            message_id: Uuid::new_v4(),
            author_id: recipient,
            content: "secret".into(),
            dm_member_ids: Some(vec![recipient]),
        };

        assert!(should_deliver(
            &dm_event,
            server_id,
            Some(Uuid::new_v4()),
            recipient
        ));
        assert!(!should_deliver(
            &dm_event,
            server_id,
            Some(Uuid::new_v4()),
            outsider
        ));

        let reactions_event = WsEvent::MessageReactionsUpdated {
            server_id: None,
            channel_id,
            message_id: Uuid::new_v4(),
            dm_member_ids: Some(vec![recipient]),
        };
        assert!(should_deliver(
            &reactions_event,
            server_id,
            Some(Uuid::new_v4()),
            recipient
        ));
        assert!(!should_deliver(
            &reactions_event,
            server_id,
            Some(Uuid::new_v4()),
            outsider
        ));
    }
}
