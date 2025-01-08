//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
#![cfg_attr(all(not(feature = "std"), not(test)), no_std)]
#![cfg_attr(not(test), deny(clippy::disallowed_methods))]
#![cfg_attr(test, allow(clippy::disallowed_methods))]
#![deny(clippy::disallowed_types)]

#[cfg(feature = "mqttv3")]
pub mod v3;

#[cfg(feature = "mqttv5")]
pub mod v5;

#[cfg(feature = "std")]
pub mod topic;
