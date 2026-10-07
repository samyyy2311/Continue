// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import {
  BatteryCharging,
  BatteryFull,
  BatteryLow,
  BatteryMedium,
  Check,
  File,
  FileArchive,
  FileAudio,
  FileCode,
  FileImage,
  FileText,
  FileVideo,
  Signal,
  SignalHigh,
  SignalLow,
  SignalMedium,
  SignalZero,
  Wifi,
  WifiHigh,
  WifiLow,
  WifiZero,
} from "lucide-react";
import { getFileCategory } from "./format.ts";
import type { TrustedPeer } from "./types.ts";

const SIGNAL_ICONS = [SignalZero, SignalLow, SignalMedium, SignalHigh, Signal];
const WIFI_ICONS = [WifiZero, WifiLow, WifiLow, WifiHigh, Wifi];

export function ConnectionStatus({ peer }: { peer: TrustedPeer }) {
  const { isConnected, status } = peer;
  const percent = status?.percent ?? 0;
  const BatteryIcon = status?.charging
    ? BatteryCharging
    : percent <= 20
      ? BatteryLow
      : percent <= 60
        ? BatteryMedium
        : BatteryFull;
  const CellIcon = status?.cellBars != null ? SIGNAL_ICONS[status.cellBars] : null;
  const WifiIcon = status?.wifiBars != null ? WIFI_ICONS[status.wifiBars] : null;
  return (
    <span className={`status ${isConnected ? "online" : ""}`}>
      {isConnected ? "Connected" : "Not connected"}
      {isConnected && status && (
        <span className="device-meters">
          <span aria-label={`Battery ${status.percent}%${status.charging ? ", charging" : ""}`}>
            <BatteryIcon size={16} aria-hidden="true" />
            {status.percent}%
          </span>
          {CellIcon && (
            <span title={status.carrier} aria-label={`${status.carrier || "Mobile"} signal ${status.cellBars} of 4`}>
              <CellIcon size={16} aria-hidden="true" />
              {status.network}
            </span>
          )}
          {WifiIcon && (
            <span aria-label={`Wi-Fi signal ${status.wifiBars} of 4`}>
              <WifiIcon size={16} aria-hidden="true" />
            </span>
          )}
        </span>
      )}
    </span>
  );
}

/** Linear progress. Without a known size it shows a moving segment. */
export function ProgressBar(props: { value: number | null; label: string }) {
  const { value, label } = props;
  const percent = value === null ? null : Math.round(Math.max(0, Math.min(1, value)) * 100);
  return (
    <div
      className={`progress ${percent === null ? "indeterminate" : ""}`}
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={percent ?? undefined}
    >
      <span style={percent === null ? undefined : { width: `${percent}%` }} />
    </div>
  );
}

/** One choice from a few, shown as joined buttons. */
export function ButtonGroup<T extends string>(props: {
  label: string;
  options: readonly { value: T; label: string }[];
  value: T;
  onChange: (value: T) => void;
}) {
  const { label, options, value, onChange } = props;
  return (
    <div className="button-group" role="radiogroup" aria-label={label}>
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          role="radio"
          aria-checked={value === option.value}
          onClick={() => onChange(option.value)}
        >
          {value === option.value && <Check size={16} aria-hidden="true" />}
          {option.label}
        </button>
      ))}
    </div>
  );
}

/** An on/off switch. */
export function Switch(props: { checked: boolean; onChange: (checked: boolean) => void; labelledBy: string }) {
  const { checked, onChange, labelledBy } = props;
  return (
    <button
      type="button"
      role="switch"
      className="switch"
      aria-checked={checked}
      aria-labelledby={labelledBy}
      onClick={() => onChange(!checked)}
    >
      <span />
    </button>
  );
}

export function getFileIcon(name: string) {
  const category = getFileCategory(name);
  switch (category) {
    case "image":
      return <FileImage size={18} />;
    case "video":
      return <FileVideo size={18} />;
    case "audio":
      return <FileAudio size={18} />;
    case "archive":
      return <FileArchive size={18} />;
    case "code":
      return <FileCode size={18} />;
    case "document":
      return <FileText size={18} />;
    default:
      return <File size={18} />;
  }
}
