// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useState } from "react";
import { Copy, Download, MessageSquareText, Phone, Search, User } from "lucide-react";
import { callNumber, errorMessage, getPhoneFile, searchPhone } from "./api.ts";
import { getFileIcon } from "./components.tsx";
import { formatRelativeTime } from "./format.ts";
import type { SearchResult } from "./types.ts";

/** Waits for typing to pause, so each key doesn't send a search. */
const TYPING_PAUSE_MS = 300;

/** The phone's files, texts and contacts. The phone does the looking; only matches come back. */
export function PhoneSearch(props: { peer: string; onError: (message: string) => void }) {
  const { peer, onError } = props;
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[] | null>(null);

  useEffect(() => {
    const wanted = query.trim();
    if (!wanted) {
      setResults(null);
      return;
    }
    let current = true;
    const timer = setTimeout(() => {
      searchPhone(peer, wanted)
        .then((found) => current && setResults(found))
        .catch((error) => current && onError(errorMessage(error)));
    }, TYPING_PAUSE_MS);
    return () => {
      current = false;
      clearTimeout(timer);
    };
  }, [peer, query, onError]);

  const fetchFile = (path: string) => getPhoneFile(peer, path).catch((error) => onError(errorMessage(error)));

  return (
    <>
      <label className="search">
        <Search size={20} />
        <input
          autoFocus
          type="search"
          placeholder="Files, texts and contacts on your phone"
          aria-label="Search your phone"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      </label>
      {results && results.length === 0 && <p className="supporting">Nothing on your phone matches.</p>}
      {results && results.length > 0 && (
        <ul className="list">
          {results.map((result, index) => (
            <li key={`${result.kind}-${result.reference}-${index}`} className="list-item">
              <span className="list-leading">
                {result.kind === "file" ? (
                  getFileIcon(result.title)
                ) : result.kind === "text" ? (
                  <MessageSquareText size={18} />
                ) : (
                  <User size={18} />
                )}
              </span>
              <div className="list-text">
                <span className="list-title" title={result.title}>
                  {result.title}
                </span>
                <span className="list-sub" title={result.detail}>
                  {result.detail}
                  {result.at > 0 && `, ${formatRelativeTime(result.at)}`}
                </span>
              </div>
              <div className="list-trailing">
                {result.kind === "file" && (
                  <button
                    type="button"
                    className="icon-btn"
                    title="Get it on this computer"
                    onClick={() => void fetchFile(result.reference)}
                  >
                    <Download size={18} />
                  </button>
                )}
                {result.kind === "contact" && (
                  <button
                    type="button"
                    className="icon-btn"
                    title="Call from your phone"
                    onClick={() => callNumber(peer, result.detail).catch((error) => onError(errorMessage(error)))}
                  >
                    <Phone size={18} />
                  </button>
                )}
                {result.kind !== "file" && (
                  <button
                    type="button"
                    className="icon-btn"
                    title="Copy"
                    onClick={() => void navigator.clipboard.writeText(result.detail)}
                  >
                    <Copy size={18} />
                  </button>
                )}
              </div>
            </li>
          ))}
        </ul>
      )}
    </>
  );
}
