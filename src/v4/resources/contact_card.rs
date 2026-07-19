//! V4 contact-card operations.
#![allow(missing_docs)]

use super::impl_v4_operation;
use crate::v4::types::{ActionResponse, ContactCard, ItemEnvelope};
use crate::{
    Result,
    core::{
        multipart,
        operation::{encode_path_segment, json_body},
    },
};
use http::Method;
use serde_json::Value;

/// Get a sender number's contact card.
#[derive(Debug, Clone)]
pub struct GetContactCard {
    pub number: String,
}
impl crate::Operation for GetContactCard {
    type Output = ItemEnvelope<ContactCard>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!(
            "/me/numbers/{}/contact-card",
            encode_path_segment(&self.number)
        )
    }
}
impl_v4_operation!(GetContactCard);

/// Update a sender number's contact card.
#[derive(Debug, Clone)]
pub struct UpdateContactCard {
    pub number: String,
    pub fields: Value,
}
impl crate::Operation for UpdateContactCard {
    type Output = ItemEnvelope<ContactCard>;
    const METHOD: Method = Method::PUT;
    fn path(&self) -> String {
        format!(
            "/me/numbers/{}/contact-card",
            encode_path_segment(&self.number)
        )
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(&self.fields)
    }
}
impl_v4_operation!(UpdateContactCard);

/// Upload a contact-card avatar.
#[derive(Debug, Clone)]
pub struct UploadContactCardAvatar {
    pub number: String,
    avatar: Vec<u8>,
    filename: String,
    content_type: String,
}
impl UploadContactCardAvatar {
    /// Construct an avatar upload.
    #[must_use]
    pub fn new(number: impl Into<String>, avatar: impl Into<Vec<u8>>) -> Self {
        Self {
            number: number.into(),
            avatar: avatar.into(),
            filename: "avatar.jpg".into(),
            content_type: "image/jpeg".into(),
        }
    }
}
impl crate::Operation for UploadContactCardAvatar {
    type Output = ActionResponse;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        format!(
            "/me/numbers/{}/contact-card/avatar",
            encode_path_segment(&self.number)
        )
    }
    fn headers(&self) -> Vec<(&'static str, String)> {
        let boundary = multipart::boundary_for(&self.avatar);
        vec![(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )]
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        let boundary = multipart::boundary_for(&self.avatar);
        let content_type = multipart::part_content_type(Some(&self.content_type))?;
        Ok(Some(multipart::file_body(
            &boundary,
            "avatar",
            &self.avatar,
            Some(&self.filename),
            content_type,
        )))
    }
}
impl_v4_operation!(UploadContactCardAvatar);

/// Contact-card resource handle.
#[derive(Debug)]
pub struct ContactCards<C> {
    pub(crate) client: C,
}
#[cfg(feature = "async")]
impl crate::v4::BlooioAccount<'_> {
    /// Access contact cards.
    #[must_use]
    pub fn contact_cards(self) -> ContactCards<Self> {
        ContactCards { client: self }
    }
}
#[cfg(feature = "sync")]
impl crate::v4::BlockingBlooioAccount<'_> {
    /// Access contact cards.
    #[must_use]
    pub fn contact_cards(self) -> ContactCards<Self> {
        ContactCards { client: self }
    }
}
#[cfg(feature = "async")]
impl ContactCards<crate::v4::BlooioAccount<'_>> {
    /// Get a number's contact card.
    pub async fn get(&self, number: impl Into<String>) -> Result<ItemEnvelope<ContactCard>> {
        self.client
            .send(GetContactCard {
                number: number.into(),
            })
            .await
    }
    /// Update a number's contact card.
    pub async fn update(&self, operation: UpdateContactCard) -> Result<ItemEnvelope<ContactCard>> {
        self.client.send(operation).await
    }
    /// Upload a number's contact-card avatar.
    pub async fn upload_avatar(
        &self,
        operation: UploadContactCardAvatar,
    ) -> Result<ActionResponse> {
        self.client.send(operation).await
    }
}
#[cfg(feature = "sync")]
impl ContactCards<crate::v4::BlockingBlooioAccount<'_>> {
    /// Get a number's contact card.
    pub fn get(&self, number: impl Into<String>) -> Result<ItemEnvelope<ContactCard>> {
        self.client.send(GetContactCard {
            number: number.into(),
        })
    }
    /// Update a number's contact card.
    pub fn update(&self, operation: UpdateContactCard) -> Result<ItemEnvelope<ContactCard>> {
        self.client.send(operation)
    }
    /// Upload a number's contact-card avatar.
    pub fn upload_avatar(&self, operation: UploadContactCardAvatar) -> Result<ActionResponse> {
        self.client.send(operation)
    }
}
