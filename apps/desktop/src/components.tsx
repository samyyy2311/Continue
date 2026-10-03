// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import type { ReactNode } from "react";

export function DeviceGlyph(props: { icon: ReactNode; active?: boolean; size?: "md" | "lg" }) {
  const { icon, active = false, size = "md" } = props;
  return (
    <span className={`glyph glyph-${size} ${active ? "active" : ""}`} aria-hidden="true">
      {icon}
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
          {option.label}
        </button>
      ))}
    </div>
  );
}
