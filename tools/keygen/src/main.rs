// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use crypto::keys::{generate_ed25519_seed, signing_key_from_seed, verifying_key};
use identity::Fingerprint;

fn main() {
    let seed = generate_ed25519_seed();
    let signing = signing_key_from_seed(&seed.0);
    let verifying = verifying_key(&signing);
    let pubkey_bytes = verifying.to_bytes();
    let fingerprint = Fingerprint::from_verifying_key(&verifying);

    println!("Ed25519 Seed (hex):        {}", hex::encode(&seed.0[..]));
    println!("Public Key (hex):          {}", hex::encode(pubkey_bytes));
    println!("Public Key (base64url):    {}", URL_SAFE_NO_PAD.encode(pubkey_bytes));
    println!("Device Fingerprint:        {}", fingerprint);
}

mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        let mut s = String::new();
        for b in bytes.as_ref() {
            use std::fmt::Write;
            let _ = write!(&mut s, "{:02x}", b);
        }
        s
    }
}
