//! Contact card: get and update the user's contact card for a phone number.

use http::Method;
use serde::{Deserialize, Serialize};

use crate::core::operation::{Operation, json_body};
use crate::error::Result;

// ---------------------------------------------------------------------------
// Shared sub-type (used in both request and response)
// ---------------------------------------------------------------------------

/// Sharing settings for a contact card.
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContactCardSharing {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_format: Option<i64>,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Response of `GET /me/numbers/{number}/contact-card`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct MyContactCard {
    pub phone_number: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub has_wallpaper: Option<bool>,
    pub sharing: Option<ContactCardSharing>,
}

/// Response of `PUT /me/numbers/{number}/contact-card`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct UpdateContactCardResponse {
    pub success: Option<bool>,
    pub phone_number: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

/// `GET /me/numbers/{number}/contact-card`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct GetMyContactCard {
    pub number: String,
}

impl Operation for GetMyContactCard {
    type Output = MyContactCard;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!("/me/numbers/{}/contact-card", self.number)
    }
}

/// `PUT /me/numbers/{number}/contact-card`
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize)]
pub struct UpdateMyContactCard {
    #[serde(skip)]
    pub number: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sharing: Option<ContactCardSharing>,
}

impl UpdateMyContactCard {
    /// Create a new update operation for the given number.
    pub fn new(number: impl Into<String>) -> Self {
        Self {
            number: number.into(),
            first_name: None,
            last_name: None,
            avatar: None,
            sharing: None,
        }
    }

    /// Set the first name.
    #[must_use]
    pub fn first_name(mut self, v: impl Into<String>) -> Self {
        self.first_name = Some(v.into());
        self
    }

    /// Set the last name.
    #[must_use]
    pub fn last_name(mut self, v: impl Into<String>) -> Self {
        self.last_name = Some(v.into());
        self
    }

    /// Set the avatar URL or data.
    #[must_use]
    pub fn avatar(mut self, v: impl Into<String>) -> Self {
        self.avatar = Some(v.into());
        self
    }

    /// Set sharing settings.
    #[must_use]
    pub fn sharing(mut self, v: ContactCardSharing) -> Self {
        self.sharing = Some(v);
        self
    }
}

impl Operation for UpdateMyContactCard {
    type Output = UpdateContactCardResponse;
    const METHOD: Method = Method::PUT;
    fn path(&self) -> String {
        format!("/me/numbers/{}/contact-card", self.number)
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}

// ---------------------------------------------------------------------------
// Resource handle + accessors.
// ---------------------------------------------------------------------------

/// Handle for the `contact_card` resource group. Created via
/// [`Client::contact_card`](crate::Client::contact_card).
#[derive(Debug)]
pub struct ContactCard<'c, C> {
    pub(crate) client: &'c C,
}

#[cfg(feature = "async")]
impl crate::Client {
    /// Access the `contact_card` resource group.
    pub fn contact_card(&self) -> ContactCard<'_, crate::Client> {
        ContactCard { client: self }
    }
}

#[cfg(feature = "sync")]
impl crate::BlockingClient {
    /// Access the `contact_card` resource group.
    pub fn contact_card(&self) -> ContactCard<'_, crate::BlockingClient> {
        ContactCard { client: self }
    }
}

#[cfg(feature = "async")]
impl ContactCard<'_, crate::Client> {
    /// Get the contact card for a phone number.
    pub async fn get(&self, number: impl Into<String>) -> Result<MyContactCard> {
        self.client
            .send(GetMyContactCard {
                number: number.into(),
            })
            .await
    }

    /// Update the contact card for a phone number.
    pub async fn update(&self, op: UpdateMyContactCard) -> Result<UpdateContactCardResponse> {
        self.client.send(op).await
    }
}

#[cfg(feature = "sync")]
impl ContactCard<'_, crate::BlockingClient> {
    /// Get the contact card for a phone number.
    pub fn get(&self, number: impl Into<String>) -> Result<MyContactCard> {
        self.client.send(GetMyContactCard {
            number: number.into(),
        })
    }

    /// Update the contact card for a phone number.
    pub fn update(&self, op: UpdateMyContactCard) -> Result<UpdateContactCardResponse> {
        self.client.send(op)
    }
}
