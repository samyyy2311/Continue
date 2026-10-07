// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useState } from "react";
import { Check, ChevronRight, Download, Folder } from "lucide-react";
import { errorMessage, getPhoneFile, listPhoneFolder } from "./api.ts";
import { getFileIcon } from "./components.tsx";
import { formatBytes, formatRelativeTime } from "./format.ts";
import type { PhoneFile } from "./types.ts";

export function PhoneFiles({ peer, onError }: { peer: string; onError: (message: string) => void }) {
  const [path, setPath] = useState<string[]>([]);
  // Undefined while asking; null when the phone isn't sharing its files.
  const [entries, setEntries] = useState<PhoneFile[] | null>();
  const [problem, setProblem] = useState<string | null>(null);
  const [copied, setCopied] = useState<Set<string>>(new Set());

  const folder = path.join("/");
  useEffect(() => {
    let current = true;
    setEntries(undefined);
    setProblem(null);
    listPhoneFolder(peer, folder).then(
      (found) => current && setEntries(found),
      (error) => current && setProblem(errorMessage(error)),
    );
    return () => {
      current = false;
    };
  }, [peer, folder]);

  const copy = (name: string) => {
    const file = [...path, name].join("/");
    getPhoneFile(peer, file).then(
      () => setCopied((done) => new Set(done).add(file)),
      (error) => onError(errorMessage(error)),
    );
  };

  let note: string | null = null;
  if (problem) note = problem;
  else if (entries === undefined) note = "Asking your phone…";
  else if (entries === null)
    note = "Turn on Browse phone files from your computer in Continue's settings on your phone.";
  else if (entries.length === 0) note = "This folder is empty.";

  return (
    <div className="phone-files">
      <nav className="crumbs" aria-label="Folder">
        {["Phone", ...path].map((name, depth) => (
          <span key={depth} className="crumb">
            {depth > 0 && <ChevronRight size={16} aria-hidden="true" />}
            <button
              type="button"
              aria-current={depth === path.length ? "location" : undefined}
              onClick={() => setPath(path.slice(0, depth))}
            >
              {name}
            </button>
          </span>
        ))}
      </nav>

      {note ? (
        <p className="supporting">{note}</p>
      ) : (
        <ul className="list">
          {entries?.map((entry) => {
            const done = copied.has([...path, entry.name].join("/"));
            return (
              <li key={entry.name} className="list-item">
                <span className="list-leading">{entry.folder ? <Folder size={18} /> : getFileIcon(entry.name)}</span>
                {entry.folder ? (
                  <button type="button" className="list-text file-open" onClick={() => setPath([...path, entry.name])}>
                    <span className="list-title">{entry.name}</span>
                    <span className="list-sub">{formatRelativeTime(entry.modified)}</span>
                  </button>
                ) : (
                  <div className="list-text">
                    <span className="list-title">{entry.name}</span>
                    <span className="list-sub">
                      {formatBytes(entry.size)} · {formatRelativeTime(entry.modified)}
                    </span>
                  </div>
                )}
                {!entry.folder && (
                  <button
                    type="button"
                    className="icon-btn"
                    title={done ? "Copying to this computer" : "Copy to this computer"}
                    disabled={done}
                    onClick={() => copy(entry.name)}
                  >
                    {done ? <Check size={18} /> : <Download size={18} />}
                  </button>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
