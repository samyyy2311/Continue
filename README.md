<!-- SPDX-FileCopyrightText: Contributors to the Continue project -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Continue

Continue is an open-source, local-first device continuity tool. It lets your phones, tablets, and computers work together across local networks without user accounts, third-party cloud servers, or platform lock-in.

## What it does

Continue focuses on everyday cross-device features built directly on top of secure, peer-to-peer connections:

* **File Transfer**: Stream files directly between devices over QUIC streams. Includes chunked streaming, SHA-256 verification, and path traversal protection.
* **Clipboard Sync**: Keep text and image clipboards synchronized in real time, with built-in echo suppression to stop copy loops.
* **Notification Forwarding**: Mirror incoming notifications to your desktop with quick actions and dismiss syncing.
* **Local Discovery and Pairing**: Find nearby devices automatically using mDNS with rotating ephemeral IDs to prevent tracking. Pair once with a QR code handshake verified through mutual TLS and cryptographic transcripts.

## Security and Privacy

* **Local-first by design**: All traffic stays on your local network. No external relay or cloud server is required for everyday operation.
* **Mutual authentication**: Connections run over QUIC using TLS 1.3 with custom certificate pinning. Self-signed transport certificates are tied to long-term Ed25519 device identities.
* **No tracking**: Discovery advertisements use rotating ephemeral IDs so devices cannot be passively tracked on local networks.
* **Four-layer permissions**: Every action is checked across four gates before running: platform availability, application permissions, peer trust level, and session negotiation.
* **Strict resource limits**: All protocol messages and file payloads are bounded by strict size limits to prevent denial-of-service and memory exhaustion.

## Workspace Layout

The core logic is written in Rust and split into small, focused crates:

| Crate | Purpose |
| --- | --- |
| `crates/limits` | Fixed frame limits, chunk sizes, and protocol timeouts |
| `crates/crypto` | Cryptographic primitives (Ed25519, X25519, HKDF, ChaCha20-Poly1305) |
| `crates/identity` | Device keys, identity signing, and fingerprint formats |
| `crates/protocol` | Protobuf definitions and framed message encoding |
| `crates/transport` | QUIC transport layer (Quinn) and custom TLS certificate verifiers |
| `crates/discovery` | Local mDNS advertiser and browser with rotating privacy IDs |
| `crates/pairing` | QR-based pairing state machine, replay defense, and trust store |
| `crates/sessions` | Connection lifecycle, keepalives, and reconnection backoff |
| `crates/capabilities` | Multi-layer capability evaluation and runtime negotiation |
| `crates/permissions` | Persistent per-peer permissions and access grants |
| `crates/transfer` | Streaming file transfer implementation and path sanitizers |
| `crates/clipboard` | Real-time clipboard synchronization and loop prevention |
| `crates/notifications` | Notification event dispatching, dismissal, and action handling |
| `crates/ffi` | UniFFI layer generating language bindings for desktop and mobile |
| `tests/peer` | Headless test peer and automated fault injection suite |

## Getting Started

### Prerequisites

* Rust 1.80 or newer
* Protocol Buffers compiler (`protoc`)

### Building

Clone the repository and build the workspace:

```bash
git clone https://github.com/samyyy2311/Continue.git
cd Continue
cargo build --workspace
```

### Running Tests

Run the workspace unit and integration tests:

```bash
cargo test --workspace
```

Run the automated fault-injection and pairing verification suite:

```bash
cargo run -p peer -- --test-suite
```

## License

* Protocol SDK, core crates, and `.proto` files are licensed under Apache-2.0.
* Client applications are licensed under GPL-3.0.

See [REUSE.toml](REUSE.toml) for machine-readable licensing details.
