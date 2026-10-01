// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use identity::{IdentityError, IdentitySigner, SecretStore};
use transport::TransportCertificate;
use zeroize::Zeroizing;

use crate::error::PairingError;

const IDENTITY_LABEL: &str = "device_identity";
const TRANSPORT_CERT_LABEL: &str = "transport_cert";

/// The two keys paired peers pin this device by. Both must stay the same across
/// restarts or every pairing breaks.
pub struct DeviceKeys {
    pub identity_signer: Arc<dyn IdentitySigner>,
    pub transport_cert: Arc<TransportCertificate>,
}

impl DeviceKeys {
    /// Loads the device keys from `store`, creating and saving them on first run.
    pub fn load_or_create(store: &dyn SecretStore) -> Result<Self, PairingError> {
        let identity_signer = identity::load_or_create_identity(store, IDENTITY_LABEL)?;

        let transport_cert = match store
            .load(TRANSPORT_CERT_LABEL)
            .map_err(IdentityError::from)?
        {
            Some(der) => TransportCertificate::from_pem_bytes(&der)?,
            None => {
                let cert = TransportCertificate::generate()?;
                store
                    .store(TRANSPORT_CERT_LABEL, Zeroizing::new(cert.pkcs8_der.clone()))
                    .map_err(IdentityError::from)?;
                cert
            }
        };

        Ok(Self {
            identity_signer: Arc::from(identity_signer),
            transport_cert: Arc::new(transport_cert),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use identity::FileSecretStore;

    #[test]
    fn keys_survive_a_restart() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("continue_device_keys_{nanos}"));
        let store = FileSecretStore::new(&dir).unwrap();

        let first = DeviceKeys::load_or_create(&store).unwrap();
        let second = DeviceKeys::load_or_create(&store).unwrap();
        assert_eq!(
            first.identity_signer.verifying_key().unwrap(),
            second.identity_signer.verifying_key().unwrap()
        );
        assert_eq!(
            first.transport_cert.spki_hash,
            second.transport_cert.spki_hash
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
