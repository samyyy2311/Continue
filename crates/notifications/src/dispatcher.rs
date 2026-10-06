// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use capabilities::{evaluate_capability, CapabilityQuery};
use limits::{MAX_FRAME_NOTIFICATION_BYTES, MAX_NOTIFICATION_BODY_BYTES};
use protocol::v1::notification_message::Body;
use protocol::v1::{NotificationAck, NotificationMessage};
use transport::{read_msg, write_msg};

use crate::error::NotificationError;

/// Sends a notification, or a reply to or dismissal of one, and waits for the peer to take it.
pub async fn send(
    send_stream: &mut quinn::SendStream,
    recv_stream: &mut quinn::RecvStream,
    body: Body,
    query: &CapabilityQuery,
) -> Result<(), NotificationError> {
    evaluate_capability(query)?;
    check_size(&body)?;
    let message = NotificationMessage { body: Some(body) };
    write_msg(send_stream, &message, MAX_FRAME_NOTIFICATION_BYTES).await?;
    let ack: NotificationAck = read_msg(recv_stream, MAX_FRAME_NOTIFICATION_BYTES).await?;
    if !ack.handled {
        return Err(NotificationError::ActionFailed(
            "The other device didn't take it".to_string(),
        ));
    }
    Ok(())
}

/// Reads what the peer sent. Answer it with `acknowledge` once it's been dealt with.
pub async fn read(recv_stream: &mut quinn::RecvStream) -> Result<Body, NotificationError> {
    let message: NotificationMessage = read_msg(recv_stream, MAX_FRAME_NOTIFICATION_BYTES).await?;
    let body = message
        .body
        .ok_or_else(|| NotificationError::ActionFailed("Empty notification message".to_string()))?;
    check_size(&body)?;
    Ok(body)
}

/// Tells the sender whether what it sent was taken.
pub async fn acknowledge(
    send_stream: &mut quinn::SendStream,
    handled: bool,
) -> Result<(), NotificationError> {
    let ack = NotificationAck {
        notification_id: String::new(),
        handled,
    };
    write_msg(send_stream, &ack, MAX_FRAME_NOTIFICATION_BYTES).await?;
    Ok(())
}

fn check_size(body: &Body) -> Result<(), NotificationError> {
    match body {
        Body::Post(post) if post.body.len() > MAX_NOTIFICATION_BODY_BYTES => {
            Err(NotificationError::BodyTooLarge {
                size: post.body.len(),
                limit: MAX_NOTIFICATION_BODY_BYTES,
            })
        }
        _ => Ok(()),
    }
}
