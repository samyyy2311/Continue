// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useState } from "react";
import { answerPermission, onPermissionRequest, onPermissionRequestClosed } from "./api.ts";
import type { PermissionAnswer, PermissionQuestion } from "./types.ts";

const KIND_PLURAL: Record<PermissionQuestion["kind"], string> = {
  file: "files",
  text: "text",
  notification: "notifications",
};

function headline(question: PermissionQuestion, name: string) {
  switch (question.kind) {
    case "file":
      return (
        <>
          {name} wants to send <span className="dialog-detail">{question.detail ?? "a file"}</span>
        </>
      );
    case "text":
      return `${name} wants to send text`;
    case "notification":
      return `${name} wants to show a notification`;
  }
}

/** Asks about things sent by devices set to Ask, one question at a time. */
export function PermissionDialog() {
  const [questions, setQuestions] = useState<PermissionQuestion[]>([]);

  useEffect(() => {
    const forget = (id: number) => setQuestions((prev) => prev.filter((q) => q.id !== id));
    const unlisteners = Promise.all([
      onPermissionRequest((question) => setQuestions((prev) => [...prev, question])),
      onPermissionRequestClosed(forget),
    ]);
    return () => {
      unlisteners.then((fns) => fns.forEach((unlisten) => unlisten())).catch(() => undefined);
    };
  }, []);

  const question = questions[0];
  if (!question) return null;

  const name = question.peerName || "A paired device";
  const answer = (choice: PermissionAnswer) => {
    setQuestions((prev) => prev.filter((q) => q.id !== question.id));
    answerPermission(question.id, choice).catch(() => undefined);
  };

  return (
    <div className="scrim">
      <div
        className="dialog dialog-small"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="permission-title"
        aria-describedby="permission-body"
      >
        <header>
          <h2 id="permission-title" className="dialog-title">
            {headline(question, name)}
          </h2>
          <p id="permission-body" className="dialog-subtitle">
            Allow it this once, or always allow {KIND_PLURAL[question.kind]} from {name}.
          </p>
        </header>
        <div className="dialog-actions">
          <button type="button" className="btn btn-text" onClick={() => answer("decline")}>
            Decline
          </button>
          <button type="button" className="btn btn-tonal" onClick={() => answer("alwaysAllow")}>
            Always allow
          </button>
          <button type="button" className="btn btn-filled" onClick={() => answer("allow")} autoFocus>
            Allow
          </button>
        </div>
      </div>
    </div>
  );
}
