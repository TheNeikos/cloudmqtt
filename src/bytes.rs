//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct MqttBytes(Vec<u8>);

impl MqttBytes {
    pub const MAX_LEN: usize = u16::MAX as usize;
}

#[derive(Debug, thiserror::Error)]
pub enum MqttBytesError {
    #[error("A vector/slice of length {} is too long, max length is {}", .0, MqttBytes::MAX_LEN)]
    TooLong(usize),
}

impl AsRef<[u8]> for MqttBytes {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl TryFrom<Vec<u8>> for MqttBytes {
    type Error = MqttBytesError;

    fn try_from(s: Vec<u8>) -> Result<Self, Self::Error> {
        if s.len() > Self::MAX_LEN {
            Err(MqttBytesError::TooLong(s.len()))
        } else {
            Ok(Self(s))
        }
    }
}

impl TryFrom<&[u8]> for MqttBytes {
    type Error = MqttBytesError;

    fn try_from(s: &[u8]) -> Result<Self, Self::Error> {
        if s.len() > Self::MAX_LEN {
            Err(MqttBytesError::TooLong(s.len()))
        } else {
            Ok(Self(s.to_vec()))
        }
    }
}
