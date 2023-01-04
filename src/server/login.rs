//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
use std::sync::Arc;

use crate::server::ClientId;

/// Errors that can occur during login
#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    /// The given password did not match
    #[error("The given password did not match")]
    InvalidPassword,
}

/// Objects that can handle authentication implement this trait
#[async_trait::async_trait]
pub trait LoginHandler {
    /// Check whether to allow this client to log in
    async fn allow_login(
        &self,
        client_id: Arc<ClientId>,
        username: Option<&str>,
        password: Option<&[u8]>,
    ) -> Result<(), LoginError>;
}

#[async_trait::async_trait]
impl LoginHandler for () {
    async fn allow_login(
        &self,
        _client_id: Arc<ClientId>,
        _username: Option<&str>,
        _password: Option<&[u8]>,
    ) -> Result<(), LoginError> {
        Ok(())
    }
}
