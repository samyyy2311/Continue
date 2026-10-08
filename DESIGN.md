# Design Direction: Continue (Focused Native Utility)

## 1. Identity & Purpose
Continue is an open, private, cross-device local continuity companion between desktop and mobile devices. It facilitates seamless file transfers, clipboard synchronization, handoff, remote desktop streaming, and device actions without third-party cloud intermediaries.

## 2. Personality & Mood
- **Calm & Unobtrusive**: Sits quietly in the system tray and desktop workspace. Does not scream for attention with decorative AI slop, neon glows, or endless pulsing dots.
- **Direct & Functional**: Every control, card, button, and indicator exists for a concrete operational action.
- **Native Precision**: Feels like a first-party platform utility on Windows and macOS. Restrained spacing, crisp borders, genuine elevation, and predictable touch/click feedback.

## 3. Typography & Hierarchy
- **Font Family**: `Instrument Sans` with fallback to native system fonts (`-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif`).
- **Data & Telemetry**: Monospace for network addresses, file sizes, fingerprint hashes, and fps/bitrate counters (`ui-monospace, "Cascadia Code", monospace`).
- **Type Scale**:
  - Display: 28px, 600 weight, -0.02em letter spacing.
  - Headline: 20px, 600 weight.
  - Title: 16px, 600 weight.
  - Body: 14px, 400 weight, 1.4 line height.
  - Caption / Detail: 12px, 500 weight.
  - Monospace Data: 12px / 13px.

## 4. Color System & Contrast (WCAG AA Compliant)
- **Surfaces**:
  - Light mode: Base `#f8fafc`, Surface `#ffffff`, Raised `#f1f5f9`, Border `#e2e8f0`.
  - Dark mode: Base `#0b0f17`, Surface `#131924`, Raised `#1c2433`, Border `#2d3748`.
- **Text & Foreground**:
  - Primary text: `#0f172a` (light) / `#f1f5f9` (dark) (Contrast > 14:1).
  - Secondary text: `#475569` (light) / `#94a3b8` (dark) (Contrast > 4.8:1, passes WCAG AA).
  - Borders & Dividers: High-definition 1px line tokens with 3:1 non-text contrast against surfaces.
- **Accent**:
  - One deliberate accent at key moments (primary send button, active navigation selection).
  - Configurable palette from existing `ACCENT_PALETTE`, defaulting to Cobalt.
- **Status Signals**:
  - Online / Active: Emerald `#16a34a` (light) / `#22c55e` (dark) with clear text label.
  - Warning / Ask: Amber `#d97706` (light) / `#f59e0b` (dark).
  - Error / Lockdown: Crimson `#dc2626` (light) / `#ef4444` (dark).

## 5. Dials
- **ENERGY**: 2 (Balanced, functional focus)
- **RHYTHM**: 2 (Structured, clear spatial hierarchy between devices, transfer queue, and capabilities)
- **MOTION**: 2 (Functional state feedback only; 120-180ms ease-out transitions on hover/focus/active; zero perpetual pulsing or floating animations)
