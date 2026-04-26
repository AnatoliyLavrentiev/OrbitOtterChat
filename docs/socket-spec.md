# RTC Socket Specification

## Endpoint
- `GET /ws?token=<jwt>&server_id=<uuid>&channel_id=<uuid?>`

## Transport
- Protocol: WebSocket
- Message format: JSON
- Event envelope: all payloads include `"event":"<snake_case_name>"`

## Authorization
- `token` is mandatory and must be a valid access token.
- User must be a member of `server_id`.
- If auth fails, handshake is rejected with an HTTP error (`401` or `403`).

## Subscription Scope
- Connection is always scoped to one `server_id`.
- If `channel_id` is provided, channel-scoped events are filtered to this channel.
- If `channel_id` is omitted, channel-scoped events from all channels of the server are delivered.

## Client -> Server Events
- `typing_start`
```json
{
  "event": "typing_start",
  "channel_id": "<uuid>"
}
```
- `typing_stop`
```json
{
  "event": "typing_stop",
  "channel_id": "<uuid>"
}
```
- `set_status`
```json
{
  "event": "set_status",
  "status": "online"
}
```
Allowed status values:
- `online`
- `away`
- `invisible`

## Server -> Client Events
- `presence_joined`
```json
{
  "event": "presence_joined",
  "server_id": "<uuid>",
  "user_id": "<uuid>",
  "connected_users": ["<uuid>"]
}
```
- `presence_left`
```json
{
  "event": "presence_left",
  "server_id": "<uuid>",
  "user_id": "<uuid>",
  "connected_users": ["<uuid>"]
}
```
- `status_updated`
```json
{
  "event": "status_updated",
  "server_id": "<uuid>",
  "user_id": "<uuid>",
  "status": "away"
}
```
- `typing_start`
```json
{
  "event": "typing_start",
  "server_id": "<uuid>",
  "channel_id": "<uuid>",
  "user_id": "<uuid>"
}
```
- `typing_stop`
```json
{
  "event": "typing_stop",
  "server_id": "<uuid>",
  "channel_id": "<uuid>",
  "user_id": "<uuid>"
}
```
- `message_new`
```json
{
  "event": "message_new",
  "server_id": "<uuid>",
  "channel_id": "<uuid>",
  "message_id": "<uuid>",
  "author_id": "<uuid>",
  "content": "<text>"
}
```
- `message_deleted`
```json
{
  "event": "message_deleted",
  "server_id": "<uuid>",
  "channel_id": "<uuid>",
  "message_id": "<uuid>",
  "deleted_by": "<uuid>"
}
```
