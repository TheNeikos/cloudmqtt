//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

pub enum ClientIdentifier {
    MinimalRequired(MinimalRequiredClientIdentifier),
    PotentiallyServerProvided,
    PotentiallyAccepted(PotentiallyAcceptedClientIdentifier),
}

impl ClientIdentifier {
    pub fn new_minimal_required(s: impl Into<String>) -> Result<ClientIdentifier, ()> {
        todo!()
    }

    pub fn as_str(&self) -> &str {
        todo!()
    }
}

pub struct MinimalRequiredClientIdentifier(String);
pub struct PotentiallyAcceptedClientIdentifier(String);

