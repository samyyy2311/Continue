// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import type { ReactNode } from "react";
import { Copy, Minus, Settings, Square, X } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { isTauri } from "@tauri-apps/api/core";
import { MOD_KEY, type View } from "./types.ts";

export interface Tab {
  id: View;
  label: string;
  icon: ReactNode;
  badge?: number;
}

/** The window's own title bar: navigation in the middle, window buttons on the right. Drag it to move the window. */
export function TopBar(props: {
  tabs: Tab[];
  view: View;
  onView: (view: View) => void;
  maximized: boolean;
}) {
  const { tabs, view, onView, maximized } = props;
  const win = isTauri() ? getCurrentWindow() : null;

  return (
    <header className="topbar" data-tauri-drag-region>
      <div className="topbar-brand" data-tauri-drag-region>
        <img src="/icon.svg" alt="" className="topbar-logo" />
        <span data-tauri-drag-region>Continue</span>
      </div>

      <nav className="topbar-tabs" aria-label="Main">
        {tabs.map((tab, index) => (
          <button
            key={tab.id}
            type="button"
            className="topbar-tab"
            aria-current={view === tab.id ? "page" : undefined}
            onClick={() => onView(tab.id)}
            title={`${tab.label} (${MOD_KEY}${index + 1})`}
          >
            {tab.icon}
            {tab.label}
            {!!tab.badge && <span className="topbar-badge">{tab.badge}</span>}
          </button>
        ))}
      </nav>

      <div className="topbar-end">
        <button
          type="button"
          className="topbar-icon"
          aria-current={view === "settings" ? "page" : undefined}
          onClick={() => onView("settings")}
          title={`Settings (${MOD_KEY}${tabs.length + 1})`}
          aria-label="Settings"
        >
          <Settings size={18} />
        </button>
        {win && (
          <div className="window-controls">
            <button type="button" aria-label="Minimise" onClick={() => void win.minimize()}>
              <Minus size={16} />
            </button>
            <button
              type="button"
              aria-label={maximized ? "Restore" : "Maximise"}
              onClick={() => void win.toggleMaximize()}
            >
              {maximized ? <Copy size={13} /> : <Square size={13} />}
            </button>
            <button type="button" className="close" aria-label="Close" onClick={() => void win.close()}>
              <X size={17} />
            </button>
          </div>
        )}
      </div>
    </header>
  );
}
