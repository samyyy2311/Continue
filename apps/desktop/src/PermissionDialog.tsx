// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useRef, useState } from "react";
import {
  answerPermission,
  onPermissionRequest,
  onPermissionRequestClosed,
  pendingPermissionQuestions,
} from "./api.ts";
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
    let active = true;
    const add = (incoming: PermissionQuestion[]) =>
      setQuestions((prev) => [...prev, ...incoming.filter((q) => !prev.some((p) => p.id === q.id))]);
    const forget = (id: number) => setQuestions((prev) => prev.filter((q) => q.id !== id));

    const unlisteners = Promise.all([onPermissionRequest((q) => add([q])), onPermissionRequestClosed(forget)]);
    // Anything asked before the listeners were in place.
    unlisteners
      .then(() => pendingPermissionQuestions())
      .then((pending) => active && add(pending))
      .catch(() => undefined);
    return () => {
      active = false;
      unlisteners.then((fns) => fns.forEach((unlisten) => unlisten())).catch(() => undefined);
    };
  }, []);

  const question = questions[0];
  if (!question) return null;
  return (
    <QuestionDialog
      key={question.id}
      question={question}
      onAnswered={() => setQuestions((prev) => prev.filter((q) => q.id !== question.id))}
    />
  );
}

function QuestionDialog({ question, onAnswered }: { question: PermissionQuestion; onAnswered: () => void }) {
  const ref = useRef<HTMLDialogElement>(null);
  const [failed, setFailed] = useState(false);
  const name = question.peerName || "A paired device";

  // A modal <dialog> keeps focus inside while open and hands it back when closed.
  useEffect(() => {
    const dialog = ref.current;
    dialog?.showModal();
    return () => dialog?.close();
  }, []);

  const answer = (choice: PermissionAnswer) => {
    setFailed(false);
    answerPermission(question.id, choice).then(onAnswered, () => setFailed(true));
  };

  return (
    <dialog
      ref={ref}
      className="dialog dialog-small"
      aria-labelledby="permission-title"
      aria-describedby="permission-body"
      onCancel={(event) => {
        event.preventDefault();
        answer("decline");
      }}
    >
      <header>
        <h2 id="permission-title" className="dialog-title">
          {headline(question, name)}
        </h2>
        <p id="permission-body" className="dialog-subtitle">
          Allow it this once, or always allow {KIND_PLURAL[question.kind]} from {name}.
        </p>
      </header>
      {failed && (
        <p className="error-banner" role="alert">
          Couldn't send your answer. Try again.
        </p>
      )}
      <div className="dialog-actions">
        <button type="button" className="btn btn-text" onClick={() => answer("decline")}>
          Decline
        </button>
        <button type="button" className="btn btn-tonal" onClick={() => answer("alwaysAllow")}>
          Always allow
        </button>
        <button type="button" className="btn btn-filled" onClick={() => answer("allow")}>
          Allow
        </button>
      </div>
    </dialog>
  );
}
