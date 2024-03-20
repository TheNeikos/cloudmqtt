//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

#![deny(missing_debug_implementations)]
#![deny(clippy::std_instead_of_core)]
#![deny(clippy::alloc_instead_of_core)]

pub mod bytes;
pub mod fixed_header;
pub mod integers;
pub mod level;
pub mod packets;
pub mod properties;
pub mod reason_code;
pub mod strings;
pub mod util;
pub mod variable_header;

pub type MResult<O> = winnow::PResult<O>;
