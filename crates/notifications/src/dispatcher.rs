// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use capabilities::{evaluate_capability, CapabilityQuery};
use limits::MAX_NOTIFICATION_BODY_BYTES;
use protocol::v1::{
    NotificationAck, NotificationActionInvoke, NotificationDismiss, NotificationPost,
};

use crate::error::NotificationError;
use crate::wire::{read_msg, write_msg};

pub struct NotificationDispatcher;

impl NotificationDispatcher {
    pub fn new() -> Self {
        Self
    }

    pub async fn send_post(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        post: NotificationPost,
        query: &CapabilityQuery,
    ) -> Result<NotificationAck, NotificationError> {
        evaluate_capability(query)?;

        if post.body.len() > MAX_NOTIFICATION_BODY_BYTES {
            return Err(NotificationError::BodyTooLarge {
                size: post.body.len(),
                limit: MAX_NOTIFICATION_BODY_BYTES,
            });
        }

        write_msg(send_stream, &post).await?;
        let ack: NotificationAck = read_msg(recv_stream).await?;

        if !ack.handled {
            return Err(NotificationError::ActionFailed(
                "Remote peer rejected notification post".to_string(),
            ));
        }

        Ok(ack)
    }

    pub async fn receive_post<F>(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        query: &CapabilityQuery,
        handler: F,
    ) -> Result<NotificationPost, NotificationError>
    where
        F: FnOnce(NotificationPost) -> Result<(), String>,
    {
        evaluate_capability(query)?;

        let post: NotificationPost = read_msg(recv_stream).await?;

        if post.body.len() > MAX_NOTIFICATION_BODY_BYTES {
            let _ = write_msg(
                send_stream,
                &NotificationAck {
                    notification_id: post.notification_id.clone(),
                    handled: false,
                },
            )
            .await;
            return Err(NotificationError::BodyTooLarge {
                size: post.body.len(),
                limit: MAX_NOTIFICATION_BODY_BYTES,
            });
        }

        match handler(post.clone()) {
            Ok(()) => {
                write_msg(
                    send_stream,
                    &NotificationAck {
                        notification_id: post.notification_id.clone(),
                        handled: true,
                    },
                )
                .await?;
                Ok(post)
            }
            Err(e) => {
                write_msg(
                    send_stream,
                    &NotificationAck {
                        notification_id: post.notification_id.clone(),
                        handled: false,
                    },
                )
                .await?;
                Err(NotificationError::ActionFailed(e))
            }
        }
    }

    pub async fn send_dismiss(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        dismiss: NotificationDismiss,
        query: &CapabilityQuery,
    ) -> Result<NotificationAck, NotificationError> {
        evaluate_capability(query)?;

        write_msg(send_stream, &dismiss).await?;
        let ack: NotificationAck = read_msg(recv_stream).await?;
        Ok(ack)
    }

    pub async fn receive_dismiss<F>(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        query: &CapabilityQuery,
        handler: F,
    ) -> Result<NotificationDismiss, NotificationError>
    where
        F: FnOnce(NotificationDismiss) -> Result<(), String>,
    {
        evaluate_capability(query)?;

        let dismiss: NotificationDismiss = read_msg(recv_stream).await?;
        let success = handler(dismiss.clone()).is_ok();

        write_msg(
            send_stream,
            &NotificationAck {
                notification_id: dismiss.notification_id.clone(),
                handled: success,
            },
        )
        .await?;

        Ok(dismiss)
    }

    pub async fn send_action(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        action: NotificationActionInvoke,
        query: &CapabilityQuery,
    ) -> Result<NotificationAck, NotificationError> {
        evaluate_capability(query)?;

        write_msg(send_stream, &action).await?;
        let ack: NotificationAck = read_msg(recv_stream).await?;

        if !ack.handled {
            return Err(NotificationError::ActionFailed(
                "Remote peer failed to handle notification action".to_string(),
            ));
        }

        Ok(ack)
    }

    pub async fn receive_action<F>(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        query: &CapabilityQuery,
        handler: F,
    ) -> Result<NotificationActionInvoke, NotificationError>
    where
        F: FnOnce(NotificationActionInvoke) -> Result<(), String>,
    {
        evaluate_capability(query)?;

        let action: NotificationActionInvoke = read_msg(recv_stream).await?;

        match handler(action.clone()) {
            Ok(()) => {
                write_msg(
                    send_stream,
                    &NotificationAck {
                        notification_id: action.notification_id.clone(),
                        handled: true,
                    },
                )
                .await?;
                Ok(action)
            }
            Err(e) => {
                write_msg(
                    send_stream,
                    &NotificationAck {
                        notification_id: action.notification_id.clone(),
                        handled: false,
                    },
                )
                .await?;
                Err(NotificationError::ActionFailed(e))
            }
        }
    }
}

impl Default for NotificationDispatcher {
    fn default() -> Self {
        Self::new()
    }
}
