//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::pin::Pin;

use miette::IntoDiagnostic;
use mqtt_format::v3::packet::MPacket;

pub async fn packet_to_vec(mpacket: MPacket<'_>) -> miette::Result<Vec<u8>> {
    let mut buf = vec![];
    {
        let mut cursor = futures::io::Cursor::new(&mut buf);
        mpacket
            .write_to(Pin::new(&mut cursor))
            .await
            .into_diagnostic()?;
    }
    Ok(buf)
}
