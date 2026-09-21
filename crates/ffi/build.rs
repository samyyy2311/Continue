// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

fn main() {
    uniffi::generate_scaffolding("./src/continue.udl").unwrap();
}
