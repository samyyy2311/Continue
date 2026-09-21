// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

pub mod error;
pub mod evaluator;
pub mod negotiation;

pub use error::CapabilityError;
pub use evaluator::{evaluate_capability, CapabilityQuery};
pub use negotiation::negotiate_capabilities;
pub use protocol::CapabilityId;
