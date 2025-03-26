//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Cloud not look up DNS")]
    DnsLookup(#[source] std::io::Error),

    #[error("DNS request did not return any addresses")]
    DnsNoAddrs,

    #[error("TCP Connectionn failed")]
    TcpConnect(#[source] std::io::Error),

    #[error("Failed writing internal buffer")]
    WriteBuffer(#[source] crate::codec::MqttWriterError),

    #[error("Internal channel closed")]
    InternalChannelClosed,
}
