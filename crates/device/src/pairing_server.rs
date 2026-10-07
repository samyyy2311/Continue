// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use pairing::PairingError;
use tokio::sync::oneshot;
use transport::TransportError;

use crate::{Device, PendingPair};

/// A code shown for another device to pair with, and the endpoint waiting for it. Dropping
/// it stops waiting.
pub struct PairingServer {
    pub code: String,
    endpoint: quinn::Endpoint,
    result: oneshot::Receiver<Result<PendingPair, PairingError>>,
}

impl Device {
    /// Waits on `port` (0 for any) for a device to pair with the returned code, which tells it
    /// to come to `advertise(port)`. The device that pairs is only trusted once the result is
    /// accepted, after the person has compared its code with the phone's.
    /// Must be called inside the Tokio runtime the device runs on.
    pub fn start_pairing(
        &self,
        port: u16,
        advertise: impl FnOnce(u16) -> String,
    ) -> Result<PairingServer, PairingError> {
        let their_key = Arc::new(Mutex::new(None));
        let tls = self
            .keys
            .transport_cert
            .build_pairing_server_tls(their_key.clone())?;
        let endpoint =
            transport::create_server_endpoint(SocketAddr::from(([0, 0, 0, 0], port)), tls)?;
        let bound = endpoint.local_addr().map_err(failed)?.port();
        let mut initiator = self.initiator();
        let code = initiator.generate_qr(advertise(bound))?.encode();

        let (done, result) = oneshot::channel();
        let (device, waiting) = (self.clone(), endpoint.clone());
        tokio::spawn(async move {
            let paired = async {
                let incoming = waiting
                    .accept()
                    .await
                    .ok_or_else(|| failed("Listener closed"))?;
                let connection = incoming
                    .await
                    .map_err(|e| failed(format!("Connection failed: {e}")))?;
                let (mut send, mut recv) = connection
                    .accept_bi()
                    .await
                    .map_err(|e| failed(format!("Stream accept failed: {e}")))?;
                let their_key = their_key
                    .lock()
                    .unwrap()
                    .ok_or(PairingError::SpkiMismatch)?;
                let pending = initiator
                    .complete_handshake(&mut send, &mut recv, their_key)
                    .await?;
                Ok(PendingPair {
                    device,
                    pending,
                    ip: connection.remote_address().ip(),
                })
            };
            let _ = done.send(paired.await);
        });

        Ok(PairingServer {
            code,
            endpoint,
            result,
        })
    }
}

impl PairingServer {
    /// The device that paired, once one has, waiting to be accepted.
    pub async fn finish(mut self) -> Result<PendingPair, PairingError> {
        (&mut self.result)
            .await
            .unwrap_or_else(|_| Err(failed("Pairing was cancelled")))
    }
}

impl Drop for PairingServer {
    fn drop(&mut self) {
        self.endpoint.close(0u32.into(), b"pairing_finished");
    }
}

fn failed(what: impl ToString) -> PairingError {
    TransportError::HandshakeFailed(what.to_string()).into()
}
