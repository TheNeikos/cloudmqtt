//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Client not connected")]
    NotConnected,

    #[error("Client is already connected")]
    AlreadyConnected,

    #[error("Internal channel closed")]
    TokioChannel,
}
