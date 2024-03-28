//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct MqttString(String);

impl MqttString {
    pub const MAX_LEN: usize = u16::MAX as usize;
}

impl std::str::FromStr for MqttString {
    type Err = MqttStringError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() > Self::MAX_LEN {
            Err(MqttStringError::TooLong(s.len()))
        } else {
            Ok(Self(s.to_string()))
        }
    }
}

impl TryFrom<&str> for MqttString {
    type Error = MqttStringError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        <MqttString as std::str::FromStr>::from_str(s)
    }
}

impl TryFrom<String> for MqttString {
    type Error = MqttStringError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.len() > Self::MAX_LEN {
            Err(MqttStringError::TooLong(s.len()))
        } else {
            Ok(Self(s.to_string()))
        }
    }
}

impl AsRef<str> for MqttString {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl std::fmt::Debug for MqttString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MqttStringError {
    #[error("String of length {} is too long, max length is {}", .0, MqttString::MAX_LEN)]
    TooLong(usize),
}
