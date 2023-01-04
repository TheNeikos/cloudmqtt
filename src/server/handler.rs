//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
use std::sync::Arc;

use mqtt_format::v3::{
    connect_return::MConnectReturnCode, qos::MQualityOfService,
    subscription_acks::MSubscriptionAck, subscription_request::MSubscriptionRequest,
};

use crate::server::ClientId;

/// Errors that can occur during login
#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    /// The given password did not match
    #[error("The given password did not match")]
    InvalidPassword,
}

impl LoginError {
    /// Convert the error into a rejection code
    pub fn as_rejection_code(&self) -> MConnectReturnCode {
        match self {
            LoginError::InvalidPassword => MConnectReturnCode::BadUsernamePassword,
        }
    }
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

/// A [`LoginHandler`] that simply allows all login attempts
pub struct AllowAllLogins;

#[async_trait::async_trait]
impl LoginHandler for AllowAllLogins {
    async fn allow_login(
        &self,
        _client_id: Arc<ClientId>,
        _username: Option<&str>,
        _password: Option<&[u8]>,
    ) -> Result<(), LoginError> {
        Ok(())
    }
}

/// Objects that can handle authentication implement this trait
#[async_trait::async_trait]
pub trait SubscriptionHandler: Send + Sync + 'static {
    /// Check whether to allow this client to log in
    async fn allow_subscription(
        &self,
        client_id: Arc<ClientId>,
        subscription: MSubscriptionRequest<'_>,
    ) -> MSubscriptionAck;
}

/// A [`SubscriptionHandler`] that simply allows all subscription requests
pub struct AllowAllSubscriptions;

#[async_trait::async_trait]
impl SubscriptionHandler for AllowAllSubscriptions {
    /// Check whether to allow this client to log in
    async fn allow_subscription(
        &self,
        _client_id: Arc<ClientId>,
        subscription: MSubscriptionRequest<'_>,
    ) -> MSubscriptionAck {
        match subscription.qos {
            MQualityOfService::AtMostOnce => MSubscriptionAck::MaximumQualityAtMostOnce,
            MQualityOfService::AtLeastOnce => MSubscriptionAck::MaximumQualityAtLeastOnce,
            MQualityOfService::ExactlyOnce => MSubscriptionAck::MaximumQualityExactlyOnce,
        }
    }
}
