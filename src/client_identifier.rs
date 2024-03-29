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
    pub fn new_minimal_required(
        s: impl Into<String>,
    ) -> Result<ClientIdentifier, ClientIdentifierError> {
        const ALLOWED_CHARS: &str =
            "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let s = s.into();

        let disallowed_chars = s
            .chars()
            .filter(|c| !ALLOWED_CHARS.chars().any(|allowed| allowed == *c))
            .collect::<Vec<char>>();

        if !disallowed_chars.is_empty() {
            return Err(ClientIdentifierError::MinimalNotAllowedChar(
                disallowed_chars,
            ));
        }

        if s.len() > 23 {
            return Err(ClientIdentifierError::MinimalTooLong(s.len()));
        }

        Ok(ClientIdentifier::MinimalRequired(
            MinimalRequiredClientIdentifier(s),
        ))
    }

    pub fn new_potentially_server_provided() -> ClientIdentifier {
        ClientIdentifier::PotentiallyServerProvided
    }

    pub fn new_potetially_accepted(
        s: impl Into<String>,
    ) -> Result<ClientIdentifier, ClientIdentifierError> {
        let s = s.into();
        if s.is_empty() {
            return Err(ClientIdentifierError::Zero);
        }
        crate::string::MqttString::try_from(s)
            .map(PotentiallyAcceptedClientIdentifier)
            .map(ClientIdentifier::PotentiallyAccepted)
            .map_err(ClientIdentifierError::from)
    }

    pub fn as_str(&self) -> &str {
        match self {
            ClientIdentifier::MinimalRequired(s) => s.0.as_ref(),
            ClientIdentifier::PotentiallyServerProvided => "",
            ClientIdentifier::PotentiallyAccepted(s) => s.0.as_ref(),
        }
    }
}

pub struct MinimalRequiredClientIdentifier(String);
pub struct PotentiallyAcceptedClientIdentifier(crate::string::MqttString);

#[derive(Debug, thiserror::Error)]
pub enum ClientIdentifierError {
    // I am ugly
    #[error("Minimal client identifier contains disallowed characters: {}", .0.iter().copied().map(String::from).collect::<Vec<_>>().join(", "))]
    MinimalNotAllowedChar(Vec<char>),

    #[error("Minimal client identifier contains more characters than allowed: {}", .0)]
    MinimalTooLong(usize),

    #[error("Client identifier is not allowed to be empty")]
    Zero,

    #[error(transparent)]
    String(#[from] crate::string::MqttStringError),
}
