use futures::StreamExt;
use rosu_v2::model::chat::{ChannelMessageType, ChatChannelId, ChatChannelMessage, ChatMessageId};
use rosu_v2::prelude::User;
use steel_core::chat::MessageType;
use steel_core::ipc::server::AppMessageIn;
use tokio::sync::mpsc::UnboundedSender;

use crate::core::http::api::Client;

const MESSAGE_PAGE_LIMIT: u32 = 50;
const MAX_MISSED_PAGES: usize = 6;
const CONCURRENT_CHANNEL_BACKFILLS: usize = 3;

pub fn register_user(client: &Client, tx: &UnboundedSender<AppMessageIn>, user: &User) {
    client.insert_user(user);

    let is_privileged = matches!(user.default_group.as_str(), "gmt" | "nat" | "dev");
    if is_privileged {
        tx.send(AppMessageIn::moderator_added(
            user.username.clone().into_string(),
        ))
        .unwrap_or_else(|e| log::error!("Failed to send moderator: {e}"));
    }
}

pub fn resolve_username(client: &Client, message: &ChatChannelMessage) -> String {
    match client.get_cached_user(message.sender_id) {
        Some(user) => user.username.into_string(),
        None => format!("(id={})", message.sender_id),
    }
}

pub fn convert_message(message: &ChatChannelMessage, username: &str) -> steel_core::chat::Message {
    let timestamp = chrono::DateTime::from_timestamp(
        message.timestamp.unix_timestamp(),
        message.timestamp.nanosecond(),
    )
    .unwrap_or_else(chrono::Utc::now)
    .with_timezone(&chrono::Local);

    let message_type =
        match message.is_action || matches!(message.message_type, ChannelMessageType::Action) {
            true => MessageType::Action,
            false => MessageType::Text,
        };

    steel_core::chat::Message::new(username, &message.content, message_type).with_time(timestamp)
}

fn ingest_message_batch(
    client: &Client,
    tx: &UnboundedSender<AppMessageIn>,
    channel_id: ChatChannelId,
    messages: &[ChatChannelMessage],
) -> Vec<steel_core::chat::Message> {
    for message in messages {
        if let Some(sender) = &message.sender {
            register_user(client, tx, sender);
        }
        client
            .message_ranges()
            .observe(channel_id, message.message_id);
    }
    messages
        .iter()
        .map(|message| convert_message(message, &resolve_username(client, message)))
        .collect()
}

/// XXX(TicClick): Should run before the websocket starts streaming chat events to avoid double intake.
pub async fn backfill_missed_messages(tx: &UnboundedSender<AppMessageIn>, client: &Client) {
    let channels = client.message_ranges().last_seen_ids();
    futures::stream::iter(
        channels.into_iter().map(|(channel_id, last_seen_id)| {
            backfill_channel(tx, client, channel_id, last_seen_id)
        }),
    )
    .buffer_unordered(CONCURRENT_CHANNEL_BACKFILLS)
    .collect::<Vec<()>>()
    .await;
}

async fn backfill_channel(
    tx: &UnboundedSender<AppMessageIn>,
    client: &Client,
    channel_id: ChatChannelId,
    last_seen_id: ChatMessageId,
) {
    let channel = match client.get_or_fetch_channel(channel_id).await {
        Ok(channel) => channel,
        Err(e) => {
            log::error!(
                "Skipping message backfill for channel #{channel_id}: failed to fetch channel info: {e}"
            );
            return;
        }
    };
    let channel_name = channel.name;

    // the presence data received on (re)connection contains the id of the newest message of each channel. when nothing
    // has happened, skip the channel for free.
    if let Some(server_newest_id) = channel.last_message_id {
        if server_newest_id <= last_seen_id {
            return;
        }
    }

    let (mut collected, skipped_older_messages) =
        fetch_newest_gap_pages(client, channel_id, last_seen_id).await;
    if collected.is_empty() {
        return;
    }
    collected.sort_unstable_by_key(|m| m.message_id);

    log::info!(
        "Fetched {} message(s) missed in {channel_name} while offline{}",
        collected.len(),
        match skipped_older_messages {
            true => " (the older part of the backlog was skipped)",
            false => "",
        }
    );

    let mut messages = ingest_message_batch(client, tx, channel_id, &collected);
    if skipped_older_messages {
        let notice = steel_core::chat::Message::new_system(
            "Some messages sent during the downtime were skipped -- showing only the most recent ones",
        )
        .with_time(messages[0].time);
        messages.insert(0, notice);
    }

    tx.send(AppMessageIn::missed_messages_received(
        channel_name,
        messages,
    ))
    .unwrap_or_else(|e| log::error!("Failed to send missed messages: {e}"));
}

/// Read the missed chat backwards until the budget runs out. Shorter pages do not necessaril ymean end of response.
async fn fetch_newest_gap_pages(
    client: &Client,
    channel_id: ChatChannelId,
    last_seen_id: ChatMessageId,
) -> (Vec<ChatChannelMessage>, bool) {
    let mut collected = Vec::new();
    let mut until: Option<ChatMessageId> = None;
    for page_no in 0..MAX_MISSED_PAGES {
        let batch = match client
            .chat_channel_messages(channel_id, None, until, MESSAGE_PAGE_LIMIT)
            .await
        {
            Ok(batch) => batch,
            Err(e) => {
                log::error!("Failed to fetch missed messages of channel #{channel_id}: {e}");
                return (collected, true);
            }
        };
        log::debug!(
            "Missed messages of channel #{channel_id}, page {page_no} (until = {until:?}): {} message(s)",
            batch.len()
        );

        let Some(oldest_fetched) = batch.iter().map(|m| m.message_id).min() else {
            return (collected, false);
        };

        let reached_local_history = batch.iter().any(|m| m.message_id <= last_seen_id);

        collected.extend(batch.into_iter().filter(|m| {
            m.message_id > last_seen_id && until.is_none_or(|until| m.message_id < until)
        }));

        if reached_local_history {
            return (collected, false);
        }
        until = Some(oldest_fetched);
    }
    (collected, true)
}

pub async fn fetch_older_messages(
    tx: &UnboundedSender<AppMessageIn>,
    client: &Client,
    target: String,
    channel_id: ChatChannelId,
) {
    let first_ingested_id = client.message_ranges().first_ingested(channel_id);

    let mut collected = Vec::new();
    match client
        .chat_channel_messages(channel_id, None, first_ingested_id, MESSAGE_PAGE_LIMIT)
        .await
    {
        Ok(mut batch) => {
            batch.sort_unstable_by_key(|m| m.message_id);
            if let Some(first_ingested_id) = first_ingested_id {
                batch.retain(|m| m.message_id < first_ingested_id);
            }
            collected = ingest_message_batch(client, tx, channel_id, &batch);
        }
        Err(e) => {
            log::error!("Failed to fetch older messages of {target}: {e}");
        }
    }

    tx.send(AppMessageIn::older_messages_received(target, collected))
        .unwrap_or_else(|e| log::error!("Failed to send older messages: {e}"));
}
