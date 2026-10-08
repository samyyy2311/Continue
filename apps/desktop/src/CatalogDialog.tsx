// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useState } from "react";
import { FolderOpen, FolderSync, HardDrive, Image as ImageIcon, Loader, X } from "lucide-react";
import {
  errorMessage,
  getCatalogThumbnail,
  mountCloudFiles,
  queryFileCatalog,
} from "./api.ts";
import { formatBytes, formatRelativeTime } from "./format.ts";
import {
  CatalogCategory,
  type CatalogItem,
  type TrustedPeer,
} from "./types.ts";

interface CatalogDialogProps {
  peer: TrustedPeer;
  onClose: () => void;
  onError: (message: string) => void;
}

const CATEGORIES: { id: CatalogCategory; label: string }[] = [
  { id: CatalogCategory.Photos, label: "Photos" },
  { id: CatalogCategory.Screenshots, label: "Screenshots" },
  { id: CatalogCategory.Downloads, label: "Downloads" },
  { id: CatalogCategory.Documents, label: "Documents" },
];

export function CatalogDialog({ peer, onClose, onError }: CatalogDialogProps) {
  const [activeCategory, setActiveCategory] = useState<CatalogCategory>(CatalogCategory.Photos);
  const [items, setItems] = useState<CatalogItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [mounting, setMounting] = useState(false);
  const [mountedPath, setMountedPath] = useState<string | null>(null);
  const [thumbnails, setThumbnails] = useState<Record<string, string>>({});

  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [onClose]);

  useEffect(() => {
    let active = true;
    setLoading(true);
    queryFileCatalog(peer.fingerprint, activeCategory, 60, 0)
      .then((resp) => {
        if (!active) return;
        setItems(resp.items);
        setLoading(false);
      })
      .catch((err) => {
        if (!active) return;
        setLoading(false);
        onError(errorMessage(err));
      });
    return () => {
      active = false;
    };
  }, [peer.fingerprint, activeCategory, onError]);

  useEffect(() => {
    if (activeCategory !== CatalogCategory.Photos && activeCategory !== CatalogCategory.Screenshots) {
      return;
    }
    const toLoad = items.slice(0, 16).filter((it) => !thumbnails[it.id]);
    for (const item of toLoad) {
      getCatalogThumbnail(peer.fingerprint, item.id, 128)
        .then((thumb) => {
          setThumbnails((prev) => ({ ...prev, [item.id]: thumb }));
        })
        .catch(() => {});
    }
  }, [peer.fingerprint, activeCategory, items, thumbnails]);

  const handleMount = async () => {
    setMounting(true);
    try {
      const path = await mountCloudFiles(peer.fingerprint);
      setMountedPath(path);
    } catch (err) {
      onError(errorMessage(err));
    } finally {
      setMounting(false);
    }
  };

  const isVisual =
    activeCategory === CatalogCategory.Photos || activeCategory === CatalogCategory.Screenshots;

  return (
    <div className="scrim" onMouseDown={onClose}>
      <div
        className="dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="catalog-dialog-title"
        onMouseDown={(e) => e.stopPropagation()}
        style={{ maxWidth: 720 }}
      >
        <header className="dialog-header">
          <div>
            <h2 id="catalog-dialog-title" className="dialog-title">
              Browse {peer.displayName}
            </h2>
            <p className="dialog-subtitle">Explore remote photos, downloads, and files.</p>
          </div>
          <button type="button" className="icon-btn" onClick={onClose} aria-label="Close">
            <X size={20} />
          </button>
        </header>

        <div className="toolbar" style={{ justifyContent: "space-between" }}>
          <div className="chips" role="tablist" aria-label="Catalog category">
            {CATEGORIES.map((cat) => (
              <button
                key={cat.id}
                type="button"
                role="tab"
                aria-selected={activeCategory === cat.id}
                className="chip"
                onClick={() => setActiveCategory(cat.id)}
              >
                {cat.label}
              </button>
            ))}
          </div>

          <button
            type="button"
            className="btn btn-tonal btn-small"
            onClick={handleMount}
            disabled={mounting}
          >
            {mounting ? (
              <Loader size={16} className="spin" />
            ) : mountedPath ? (
              <FolderOpen size={16} />
            ) : (
              <HardDrive size={16} />
            )}
            {mounting ? "Mounting" : mountedPath ? "Mounted in Explorer" : "Mount in Explorer"}
          </button>
        </div>

        {loading ? (
          <div style={{ display: "grid", placeItems: "center", padding: "48px 0" }}>
            <Loader size={28} className="spin" />
            <span className="supporting" style={{ marginTop: 12 }}>
              Loading items from {peer.displayName}
            </span>
          </div>
        ) : items.length === 0 ? (
          <div style={{ textAlign: "center", padding: "48px 0" }}>
            <FolderSync size={36} strokeWidth={1.25} style={{ opacity: 0.4, margin: "0 auto 8px" }} />
            <p className="supporting">No files found in this category.</p>
          </div>
        ) : isVisual ? (
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fill, minmax(130px, 1fr))",
              gap: 12,
              maxHeight: 400,
              overflowY: "auto",
            }}
          >
            {items.map((item) => {
              const thumb = thumbnails[item.id];
              return (
                <div
                  key={item.id}
                  style={{
                    display: "flex",
                    flexDirection: "column",
                    borderRadius: 12,
                    background: "var(--fill)",
                    overflow: "hidden",
                    border: "1px solid var(--line)",
                  }}
                >
                  <div
                    style={{
                      height: 100,
                      background: "var(--fill-hover)",
                      display: "grid",
                      placeItems: "center",
                    }}
                  >
                    {thumb ? (
                      <img
                        src={thumb}
                        alt={item.name}
                        style={{ width: "100%", height: "100%", objectFit: "cover" }}
                      />
                    ) : (
                      <ImageIcon size={28} style={{ opacity: 0.35 }} />
                    )}
                  </div>
                  <div style={{ padding: "8px 10px", display: "flex", flexDirection: "column" }}>
                    <span
                      style={{
                        fontSize: 12,
                        fontWeight: 600,
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                      }}
                      title={item.name}
                    >
                      {item.name}
                    </span>
                    <span className="supporting" style={{ fontSize: 11 }}>
                      {formatBytes(item.sizeBytes)}
                    </span>
                  </div>
                </div>
              );
            })}
          </div>
        ) : (
          <ul className="list" style={{ maxHeight: 400, overflowY: "auto" }}>
            {items.map((item) => (
              <li key={item.id} className="list-item">
                <span className="list-leading">
                  <FolderSync size={18} />
                </span>
                <div className="list-text">
                  <span className="list-title" title={item.name}>
                    {item.name}
                  </span>
                  <span className="list-sub">
                    {formatBytes(item.sizeBytes)}, {formatRelativeTime(item.timestamp)}
                  </span>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
