//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use nom::IResult;

pub mod connect_return;
pub mod errors;
pub mod header;
pub mod identifier;
pub mod packet;
pub mod qos;
pub mod strings;
pub mod subscription_acks;
pub mod subscription_request;
pub mod unsubscription_request;
pub mod will;

/// The result of a streaming operation
pub type MSResult<'a, T> = IResult<&'a [u8], T>;
