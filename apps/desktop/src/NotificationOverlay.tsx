// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useState } from "react";
import { MessageSquare, Send, X } from "lucide-react";
import {
  invokeNotificationAction,
  onNotificationDismissed,
  onNotificationReceived,
} from "./api.ts";
import type { NotificationAction, PhoneNotification } from "./types.ts";

export function NotificationOverlay() {
  const [notifications, setNotifications] = useState<PhoneNotification[]>([]);
  const [activeReply, setActiveReply] = useState<{
    notificationId: string;
    actionId: string;
  } | null>(null);
  const [replyText, setReplyText] = useState("");
  const [sending, setSending] = useState(false);

  useEffect(() => {
    let active = true;
    const unlisteners = Promise.all([
      onNotificationReceived((notif) => {
        if (!active) return;
        setNotifications((prev) => [
          notif,
          ...prev.filter((n) => n.notificationId !== notif.notificationId),
        ]);
      }),
      onNotificationDismissed((dismiss) => {
        if (!active) return;
        setNotifications((prev) =>
          prev.filter((n) => n.notificationId !== dismiss.notificationId),
        );
      }),
    ]);

    return () => {
      active = false;
      unlisteners.then((fns) => fns.forEach((fn) => fn())).catch(() => undefined);
    };
  }, []);

  const dismiss = (notificationId: string) => {
    setNotifications((prev) => prev.filter((n) => n.notificationId !== notificationId));
    if (activeReply?.notificationId === notificationId) {
      setActiveReply(null);
      setReplyText("");
    }
  };

  const handleAction = async (notif: PhoneNotification, action: NotificationAction) => {
    if (action.isReply) {
      if (
        activeReply?.notificationId === notif.notificationId &&
        activeReply?.actionId === action.actionId
      ) {
        setActiveReply(null);
        setReplyText("");
      } else {
        setActiveReply({ notificationId: notif.notificationId, actionId: action.actionId });
        setReplyText("");
      }
      return;
    }

    try {
      await invokeNotificationAction(notif.peerId, notif.notificationId, action.actionId, "");
      dismiss(notif.notificationId);
    } catch (e) {
      console.error("Failed to invoke notification action", e);
    }
  };

  const sendReply = async (notif: PhoneNotification, actionId: string) => {
    if (!replyText.trim() || sending) return;
    setSending(true);
    try {
      await invokeNotificationAction(
        notif.peerId,
        notif.notificationId,
        actionId,
        replyText.trim(),
      );
      dismiss(notif.notificationId);
    } catch (e) {
      console.error("Failed to send notification reply", e);
    } finally {
      setSending(false);
    }
  };

  if (notifications.length === 0) return null;

  return (
    <div className="notification-overlay" aria-live="polite">
      {notifications.map((notif) => {
        const isReplying = activeReply?.notificationId === notif.notificationId;
        return (
          <div key={notif.notificationId} className="notification-card">
            <div className="notification-card-header">
              <div className="notification-card-meta">
                <span className="notification-app-name">{notif.appName || notif.packageName}</span>
                <span className="notification-peer-name">from {notif.peerName}</span>
              </div>
              <button
                type="button"
                className="btn-icon"
                aria-label="Dismiss notification"
                onClick={() => dismiss(notif.notificationId)}
              >
                <X size={14} aria-hidden="true" />
              </button>
            </div>
            <div className="notification-card-content">
              <h4 className="notification-title">{notif.title}</h4>
              <p className="notification-body">{notif.body}</p>
            </div>
            {notif.actions.length > 0 && (
              <div className="notification-actions">
                {notif.actions.map((action) => (
                  <button
                    key={action.actionId}
                    type="button"
                    className={`btn btn-sm ${
                      activeReply?.notificationId === notif.notificationId &&
                      activeReply?.actionId === action.actionId
                        ? "btn-filled"
                        : "btn-tonal"
                    }`}
                    onClick={() => handleAction(notif, action)}
                  >
                    {action.isReply && <MessageSquare size={13} aria-hidden="true" />}
                    {action.label || (action.isReply ? "Reply" : "Open")}
                  </button>
                ))}
              </div>
            )}
            {isReplying && activeReply && (
              <form
                className="notification-reply-form"
                onSubmit={(e) => {
                  e.preventDefault();
                  sendReply(notif, activeReply.actionId);
                }}
              >
                <input
                  type="text"
                  className="notification-reply-input"
                  placeholder="Type a reply..."
                  value={replyText}
                  onChange={(e) => setReplyText(e.target.value)}
                  autoFocus
                  disabled={sending}
                />
                <button
                  type="submit"
                  className="btn btn-filled btn-sm"
                  disabled={!replyText.trim() || sending}
                  aria-label="Send reply"
                >
                  <Send size={13} aria-hidden="true" />
                </button>
              </form>
            )}
          </div>
        );
      })}
    </div>
  );
}
