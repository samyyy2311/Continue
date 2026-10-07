// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useRef, useState } from "react";
import { ArrowUp, Copy, ExternalLink, MessageSquareText, Phone, Search, SquarePen, Star, Users } from "lucide-react";
import {
  callNumber,
  errorMessage,
  listContacts,
  listConversations,
  onMessagesChanged,
  openLink,
  readConversation,
  sendSms,
} from "./api.ts";
import { formatRelativeTime, linksIn, oneTimeCode } from "./format.ts";
import type { Contact, Conversation, TextMessage } from "./types.ts";

function CallButton(props: { peer: string; number: string; onError: (message: string) => void }) {
  const { peer, number, onError } = props;
  return (
    <button
      type="button"
      className="icon-btn"
      title={`Call ${number} from your phone`}
      disabled={!number.trim()}
      onClick={() => callNumber(peer, number.trim()).catch((error) => onError(errorMessage(error)))}
    >
      <Phone size={18} />
    </button>
  );
}

/** Favourites first. Picking one starts a text to them. */
function ContactList(props: {
  peer: string;
  contacts: Contact[] | null | undefined;
  search: string;
  onText: (number: string) => void;
  onError: (message: string) => void;
}) {
  const { peer, contacts, search, onText, onError } = props;
  if (contacts === undefined) return <p className="supporting">Asking your phone…</p>;
  if (contacts === null) {
    return <p className="supporting">Turn on Text from your computer in Continue's settings on your phone.</p>;
  }
  const shown = contacts.filter(
    (c) => !search || c.name.toLowerCase().includes(search) || c.number.replace(/\s/g, "").includes(search),
  );
  return (
    <ul className="conversations" aria-label="Contacts">
      {shown.length === 0 && <li className="supporting">No contact matches.</li>}
      {shown.map((contact) => (
        <li key={`${contact.name}-${contact.number}`} className="contact">
          <button type="button" className="conversation" onClick={() => onText(contact.number)}>
            {contact.photo ? (
              <img className="avatar" src={contact.photo} alt="" />
            ) : (
              <span className="avatar" aria-hidden="true">
                {(contact.name || contact.number).replace(/^\+/, "").charAt(0).toUpperCase()}
              </span>
            )}
            <span className="conversation-text">
              <span className="conversation-head">
                <span className="conversation-name">{contact.name || contact.number}</span>
                {contact.favorite && <Star size={14} aria-label="Favourite" />}
              </span>
              <span className="conversation-snippet">{contact.number}</span>
            </span>
          </button>
          <CallButton peer={peer} number={contact.number} onError={onError} />
        </li>
      ))}
    </ul>
  );
}

/** Numbers match whatever the formatting, by their last digits. */
function sameNumber(one: string, other: string) {
  const digits = (number: string) => number.replace(/\D/g, "").slice(-9);
  return digits(one) !== "" && digits(one) === digits(other);
}

export function Messages({ peer, onError }: { peer: string; onError: (message: string) => void }) {
  // Undefined while asking; null when the phone hasn't given access.
  const [conversations, setConversations] = useState<Conversation[] | null>();
  const [problem, setProblem] = useState<string | null>(null);
  const [openId, setOpenId] = useState<string | null>(null);
  const [texts, setTexts] = useState<TextMessage[]>([]);
  const [search, setSearch] = useState("");
  const [composing, setComposing] = useState(false);
  // Who a new message is to, when started from a contact.
  const [composeTo, setComposeTo] = useState("");
  // Undefined until asked for; null when the phone hasn't given access.
  const [contacts, setContacts] = useState<Contact[] | null>();
  const [showContacts, setShowContacts] = useState(false);
  // A number just texted from New, opened once its conversation shows up.
  const [newlyTexted, setNewlyTexted] = useState<string | null>(null);

  const loadConversations = useCallback(() => {
    listConversations(peer).then(
      (found) => {
        setConversations(found);
        setProblem(null);
      },
      (error) => setProblem(errorMessage(error)),
    );
  }, [peer]);

  const loadTexts = useCallback(() => {
    if (!openId) return;
    readConversation(peer, openId).then(setTexts, (error) => onError(errorMessage(error)));
  }, [peer, openId, onError]);

  useEffect(loadConversations, [loadConversations]);
  useEffect(() => {
    if (!showContacts || contacts !== undefined) return;
    listContacts(peer).then(setContacts, (error) => onError(errorMessage(error)));
  }, [peer, showContacts, contacts, onError]);
  useEffect(loadTexts, [loadTexts]);
  useEffect(() => {
    const unlisten = onMessagesChanged((from) => {
      if (from !== peer) return;
      loadConversations();
      loadTexts();
    });
    return () => void unlisten.then((stop) => stop());
  }, [peer, loadConversations, loadTexts]);
  useEffect(() => {
    const texted = newlyTexted && conversations?.find((c) => sameNumber(c.address, newlyTexted));
    if (!texted) return;
    setNewlyTexted(null);
    setComposing(false);
    setTexts([]);
    setOpenId(texted.id);
  }, [conversations, newlyTexted]);

  let note: string | null = null;
  if (problem) note = problem;
  else if (conversations === undefined) note = "Asking your phone…";
  else if (conversations === null) note = "Turn on Text from your computer in Continue's settings on your phone.";
  if (note) return <p className="supporting messages-note">{note}</p>;

  const needle = search.trim().toLowerCase();
  const shown = (conversations ?? []).filter(
    (c) => !needle || [c.name, c.address, c.snippet].some((field) => field.toLowerCase().includes(needle)),
  );
  const open = conversations?.find((c) => c.id === openId);
  return (
    <div className="messages">
      <div className="conversation-pane">
        <div className="conversation-tools">
          <label className="search">
            <Search size={18} />
            <input
              type="search"
              placeholder="Search texts"
              aria-label="Search conversations"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
          </label>
          <button
            type="button"
            className="icon-btn"
            title={showContacts ? "Show texts" : "Show contacts"}
            aria-pressed={showContacts}
            onClick={() => setShowContacts(!showContacts)}
          >
            <Users size={18} />
          </button>
          <button
            type="button"
            className="icon-btn"
            title="New message"
            aria-pressed={composing}
            onClick={() => {
              setComposeTo("");
              setComposing(true);
            }}
          >
            <SquarePen size={18} />
          </button>
        </div>
        {showContacts && (
          <ContactList
            peer={peer}
            contacts={contacts}
            search={needle}
            onText={(number) => {
              setComposeTo(number);
              setComposing(true);
            }}
            onError={onError}
          />
        )}
        <ul className="conversations" aria-label="Conversations" hidden={showContacts}>
          {shown.length === 0 && (
            <li className="supporting">{needle ? "No conversation matches." : "No texts on your phone yet."}</li>
          )}
          {shown.map((c) => (
            <li key={c.id}>
              <button
                type="button"
                className={`conversation ${c.unread ? "unread" : ""}`}
                aria-current={!composing && c.id === openId ? "true" : undefined}
                onClick={() => {
                  setComposing(false);
                  setTexts([]);
                  setOpenId(c.id);
                }}
              >
                <span className="avatar" aria-hidden="true">
                  {(c.name || c.address).replace(/^\+/, "").charAt(0).toUpperCase()}
                </span>
                <span className="conversation-text">
                  <span className="conversation-head">
                    <span className="conversation-name">{c.name || c.address}</span>
                    <span className="conversation-time">{formatRelativeTime(c.at)}</span>
                  </span>
                  <span className="conversation-snippet">{c.snippet}</span>
                </span>
              </button>
            </li>
          ))}
        </ul>
      </div>

      {composing ? (
        <NewMessage
          key={composeTo}
          peer={peer}
          initialNumber={composeTo}
          onSent={(number) => {
            setNewlyTexted(number);
            loadConversations();
          }}
          onError={onError}
        />
      ) : open ? (
        <Thread key={open.id} peer={peer} conversation={open} texts={texts} onSent={loadTexts} onError={onError} />
      ) : (
        <div className="thread-empty">
          <MessageSquareText size={28} />
          <p className="supporting">Pick a conversation to read and reply.</p>
        </div>
      )}
    </div>
  );
}

function Composer(props: { label: string; onSend: (body: string) => Promise<boolean> }) {
  const { label, onSend } = props;
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  return (
    <form
      className="composer"
      onSubmit={(e) => {
        e.preventDefault();
        const body = draft.trim();
        if (!body) return;
        setSending(true);
        void onSend(body)
          .then((sent) => sent && setDraft(""))
          .finally(() => setSending(false));
      }}
    >
      <input
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        placeholder={label}
        aria-label={label}
        disabled={sending}
      />
      <button type="submit" className="composer-send" disabled={!draft.trim() || sending} aria-label="Send">
        <ArrowUp size={20} />
      </button>
    </form>
  );
}

function NewMessage(props: {
  peer: string;
  initialNumber: string;
  onSent: (number: string) => void;
  onError: (message: string) => void;
}) {
  const { peer, initialNumber, onSent, onError } = props;
  const [number, setNumber] = useState(initialNumber);
  const send = async (body: string) => {
    const to = number.trim();
    if (!to) {
      onError("Type who to text first.");
      return false;
    }
    try {
      await sendSms(peer, to, body);
      onSent(to);
      return true;
    } catch (error) {
      onError(errorMessage(error));
      return false;
    }
  };
  return (
    <section className="thread" aria-label="New message">
      <header className="thread-head">
        <input
          autoFocus
          className="new-message-to"
          value={number}
          onChange={(e) => setNumber(e.target.value)}
          placeholder="Phone number"
          aria-label="Phone number"
          inputMode="tel"
        />
        <CallButton peer={peer} number={number} onError={onError} />
      </header>
      <div className="bubbles" />
      <Composer label="Text message" onSend={send} />
    </section>
  );
}

function Thread(props: {
  peer: string;
  conversation: Conversation;
  texts: TextMessage[];
  onSent: () => void;
  onError: (message: string) => void;
}) {
  const { peer, conversation, texts, onSent, onError } = props;
  const end = useRef<HTMLDivElement>(null);

  useEffect(() => {
    end.current?.scrollIntoView({ block: "end" });
  }, [texts]);

  const send = async (body: string) => {
    try {
      await sendSms(peer, conversation.address, body);
      onSent();
      return true;
    } catch (error) {
      onError(errorMessage(error));
      return false;
    }
  };

  return (
    <section className="thread" aria-label={conversation.name || conversation.address}>
      <header className="thread-head">
        <h2 className="title">{conversation.name || conversation.address}</h2>
        {conversation.name && <span className="supporting">{conversation.address}</span>}
        <span className="thread-call">
          <CallButton peer={peer} number={conversation.address} onError={onError} />
        </span>
      </header>
      <div className="bubbles">
        {texts.map((text) => (
          <Bubble key={text.id} text={text} onError={onError} />
        ))}
        <div ref={end} />
      </div>
      <Composer label="Text message" onSend={send} />
    </section>
  );
}

/** A text, with its one-time code ready to copy and its links ready to open. */
function Bubble({ text, onError }: { text: TextMessage; onError: (message: string) => void }) {
  const code = text.outgoing ? null : oneTimeCode(text.body);
  const links = linksIn(text.body);
  return (
    <div className={`bubble-group ${text.outgoing ? "outgoing" : ""}`}>
      <p className={`bubble ${text.outgoing ? "outgoing" : ""}`} title={new Date(text.at).toLocaleString()}>
        {text.body}
      </p>
      {(code || links.length > 0) && (
        <div className="bubble-actions">
          {code && (
            <button type="button" className="chip" onClick={() => void navigator.clipboard.writeText(code)}>
              <Copy size={14} />
              Copy {code}
            </button>
          )}
          {links.map((link) => (
            <button
              key={link}
              type="button"
              className="chip"
              title={link}
              onClick={() => openLink(link).catch((error) => onError(errorMessage(error)))}
            >
              <ExternalLink size={14} />
              Open {new URL(link).hostname}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
