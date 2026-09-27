// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

pub fn hex_encode(bytes: impl AsRef<[u8]>) -> String {
    let mut s = String::new();
    for b in bytes.as_ref() {
        use std::fmt::Write;
        let _ = write!(&mut s, "{:02x}", b);
    }
    s
}
