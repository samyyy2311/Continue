// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { useState } from "react";
import { Send, X } from "lucide-react";
import { dismissNotification, errorMessage, pressNotificationButton } from "./api.ts";
import { formatRelativeTime } from "./format.ts";
import type { PhoneNotification } from "./types.ts";

/** The phone's notifications, newest first, with their buttons and a reply box where one takes text. */
export function NotificationList(props: { notifications: PhoneNotification[]; onError: (message: string) => void }) {
  const { notifications, onError } = props;
  return (
    <ul className="list">
      {notifications.map((notification) => (
        <NotificationRow key={notification.id} notification={notification} onError={onError} />
      ))}
    </ul>
  );
}

function NotificationRow(props: { notification: PhoneNotification; onError: (message: string) => void }) {
  const { notification, onError } = props;
  const [replyingTo, setReplyingTo] = useState<string | null>(null);
  const [reply, setReply] = useState("");
  const { peerId, id } = notification;

  const press = (buttonId: string, text = "") =>
    pressNotificationButton(peerId, id, buttonId, text).catch((error) => onError(errorMessage(error)));

  return (
    <li className="list-item notification">
      <div className="list-text">
        <span className="list-sub">
          {notification.appName}, {formatRelativeTime(notification.postedAt)}
        </span>
        <span className="list-title">{notification.title}</span>
        {notification.text && <span className="list-sub wrap">{notification.text}</span>}
        {notification.buttons.length > 0 && replyingTo === null && (
          <div className="notification-buttons">
            {notification.buttons.map((button) => (
              <button
                key={button.id}
                type="button"
                className="btn btn-tonal btn-small"
                onClick={() => (button.isReply ? setReplyingTo(button.id) : void press(button.id))}
              >
                {button.label}
              </button>
            ))}
          </div>
        )}
        {replyingTo !== null && (
          <form
            className="composer"
            onSubmit={(e) => {
              e.preventDefault();
              if (!reply.trim()) return;
              void press(replyingTo, reply.trim());
              setReplyingTo(null);
              setReply("");
            }}
          >
            <input
              autoFocus
              value={reply}
              onChange={(e) => setReply(e.target.value)}
              onKeyDown={(e) => e.key === "Escape" && setReplyingTo(null)}
              placeholder={`Reply to ${notification.title}`}
              aria-label="Reply"
            />
            <button type="submit" className="composer-send" disabled={!reply.trim()} aria-label="Send reply">
              <Send size={20} />
            </button>
          </form>
        )}
      </div>
      <button
        type="button"
        className="icon-btn"
        title="Dismiss"
        onClick={() => dismissNotification(peerId, id).catch((error) => onError(errorMessage(error)))}
      >
        <X size={18} />
      </button>
    </li>
  );
}
