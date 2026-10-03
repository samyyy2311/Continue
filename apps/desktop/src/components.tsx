// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { useId, type ReactNode } from "react";
import { Laptop, Smartphone } from "lucide-react";

/** Nine-lobed "cookie" outline from the Material 3 Expressive shape set, in a 100×100 box. */
const COOKIE_PATH = (() => {
  const points: string[] = [];
  for (let i = 0; i <= 180; i += 1) {
    const angle = (i / 180) * Math.PI * 2;
    const radius = 45 + 4 * Math.cos(9 * angle);
    points.push(`${(50 + radius * Math.cos(angle)).toFixed(2)} ${(50 + radius * Math.sin(angle)).toFixed(2)}`);
  }
  return `M${points.join("L")}Z`;
})();

export function DeviceGlyph(props: { icon: ReactNode; active?: boolean; size?: "md" | "lg" }) {
  const { icon, active = false, size = "md" } = props;
  return (
    <span className={`glyph glyph-${size} ${active ? "active" : ""}`} aria-hidden="true">
      <svg viewBox="0 0 100 100">
        <path d={COOKIE_PATH} />
      </svg>
      <span className="glyph-icon">{icon}</span>
    </span>
  );
}

/** This computer and a paired device joined by a line that flows while they're connected. */
export function LinkHero(props: { peerName: string; connected: boolean }) {
  const { peerName, connected } = props;
  return (
    <div
      className={`link-hero ${connected ? "connected" : ""}`}
      role="img"
      aria-label={`This computer and ${peerName}, ${connected ? "connected" : "not connected"}`}
    >
      <DeviceGlyph icon={<Laptop size={34} strokeWidth={1.6} />} active={connected} size="lg" />
      <svg className="link-line" viewBox="0 0 200 8" preserveAspectRatio="none" aria-hidden="true">
        <line x1="4" y1="4" x2="196" y2="4" />
      </svg>
      <DeviceGlyph icon={<Smartphone size={32} strokeWidth={1.6} />} active={connected} size="lg" />
    </div>
  );
}

/**
 * Material 3 Expressive progress: the finished part is a wave, the rest a flat track.
 * Without a known size the wave runs the full width and keeps moving.
 */
export function WavyProgress(props: { value: number | null; label: string }) {
  const { value, label } = props;
  const clipId = useId();
  const percent = value === null ? 100 : Math.max(0, Math.min(100, value * 100));
  let wave = "M0 6";
  for (let x = 0; x <= 440; x += 4) {
    wave += ` L${x} ${(6 + 3 * Math.sin((x / 400) * Math.PI * 2 * 10)).toFixed(2)}`;
  }
  return (
    <div
      className={`wavy ${value === null ? "indeterminate" : ""}`}
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={value === null ? undefined : Math.round(percent)}
    >
      <svg viewBox="0 0 400 12" preserveAspectRatio="none" aria-hidden="true">
        <line className="wavy-track" x1={Math.min(400, percent * 4 + 8)} y1="6" x2="400" y2="6" />
        <clipPath id={clipId}>
          <rect x="0" y="0" width={percent * 4} height="12" />
        </clipPath>
        <g clipPath={`url(#${clipId})`}>
          <path className="wavy-wave" d={wave} />
        </g>
      </svg>
    </div>
  );
}

/** Material 3 connected button group: one choice from a few, shown as joined pills. */
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
