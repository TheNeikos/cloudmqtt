//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

#[derive(Debug, PartialEq)]
pub enum ProposedClientIdentifier {
    MinimalRequired(MinimalRequiredClientIdentifier),
    PotentiallyServerProvided,
    PotentiallyAccepted(PotentiallyAcceptedClientIdentifier),
}

impl ProposedClientIdentifier {
    pub fn new_minimal_required(
        s: impl Into<String>,
    ) -> Result<ProposedClientIdentifier, ClientIdentifierError> {
        const ALLOWED_CHARS: &str =
            "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let s: String = s.into();

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

        crate::string::MqttString::try_from(s)
            .map(MinimalRequiredClientIdentifier)
            .map(ProposedClientIdentifier::MinimalRequired)
            .map_err(ClientIdentifierError::from)
    }

    pub fn new_potentially_server_provided() -> ProposedClientIdentifier {
        ProposedClientIdentifier::PotentiallyServerProvided
    }

    pub fn new_potetially_accepted(
        s: impl Into<String>,
    ) -> Result<ProposedClientIdentifier, ClientIdentifierError> {
        let s = s.into();
        if s.is_empty() {
            return Err(ClientIdentifierError::Zero);
        }
        crate::string::MqttString::try_from(s)
            .map(PotentiallyAcceptedClientIdentifier)
            .map(ProposedClientIdentifier::PotentiallyAccepted)
            .map_err(ClientIdentifierError::from)
    }

    pub fn as_str(&self) -> &str {
        match self {
            ProposedClientIdentifier::MinimalRequired(s) => s.0.as_ref(),
            ProposedClientIdentifier::PotentiallyServerProvided => "",
            ProposedClientIdentifier::PotentiallyAccepted(s) => s.0.as_ref(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct MinimalRequiredClientIdentifier(crate::string::MqttString);
impl MinimalRequiredClientIdentifier {
    pub fn into_inner(self) -> crate::string::MqttString {
        self.0
    }
}

#[derive(Debug, PartialEq)]
pub struct PotentiallyAcceptedClientIdentifier(crate::string::MqttString);
impl PotentiallyAcceptedClientIdentifier {
    pub fn into_inner(self) -> crate::string::MqttString {
        self.0
    }
}

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
