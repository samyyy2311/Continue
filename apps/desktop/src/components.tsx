// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { BatteryCharging, BatteryFull, BatteryLow, BatteryMedium, Check } from "lucide-react";
import type { TrustedPeer } from "./types.ts";

/** "Connected" or "Not connected", with the battery once a connected device reports it. */
export function ConnectionStatus({ peer }: { peer: TrustedPeer }) {
  const { isConnected, battery } = peer;
  const percent = battery?.percent ?? 0;
  const BatteryIcon = battery?.charging
    ? BatteryCharging
    : percent <= 20
      ? BatteryLow
      : percent <= 60
        ? BatteryMedium
        : BatteryFull;
  return (
    <span className={`status ${isConnected ? "online" : ""}`}>
      {isConnected ? "Connected" : "Not connected"}
      {isConnected && battery && (
        <span className="battery" aria-label={`Battery ${battery.percent}%${battery.charging ? ", charging" : ""}`}>
          <BatteryIcon size={16} aria-hidden="true" />
          {battery.percent}%
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
