//! Cursor pagination for v4 list endpoints.

#[cfg(any(feature = "async", feature = "sync"))]
use std::collections::HashSet;
#[cfg(any(feature = "async", feature = "sync"))]
use std::marker::PhantomData;
#[cfg(any(feature = "async", feature = "sync"))]
use std::num::NonZeroU32;

#[cfg(any(feature = "async", feature = "sync"))]
use crate::v4::Operation;
#[cfg(any(feature = "async", feature = "sync"))]
use crate::{Error, Result};

/// A page of cursor-paginated v4 results.
#[derive(Debug, Clone)]
pub struct CursorPage<T> {
    /// Items in this page.
    pub items: Vec<T>,
    /// Whether the server reports another page.
    pub has_more: bool,
    /// Opaque cursor for the next page.
    pub next_cursor: Option<String>,
}

/// Implemented by v4 cursor-list response types.
pub trait CursorListing {
    /// Element type.
    type Item;
    /// Consume the response into a cursor page.
    fn into_cursor_page(self) -> CursorPage<Self::Item>;
}

/// Lazy paginator for v4 cursor endpoints.
#[cfg(any(feature = "async", feature = "sync"))]
pub struct CursorPaginator<C, F, O>
where
    F: Fn(Option<String>, u32) -> O,
    O: Operation,
    O::Output: CursorListing,
{
    client: C,
    make: F,
    cursor: Option<String>,
    seen: HashSet<String>,
    limit: NonZeroU32,
    done: bool,
    _op: PhantomData<O>,
}

#[cfg(any(feature = "async", feature = "sync"))]
impl<C, F, O> core::fmt::Debug for CursorPaginator<C, F, O>
where
    F: Fn(Option<String>, u32) -> O,
    O: Operation,
    O::Output: CursorListing,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CursorPaginator")
            .field("has_cursor", &self.cursor.is_some())
            .field("seen_count", &self.seen.len())
            .field("limit", &self.limit)
            .field("done", &self.done)
            .finish_non_exhaustive()
    }
}

#[cfg(any(feature = "async", feature = "sync"))]
impl<C, F, O> CursorPaginator<C, F, O>
where
    F: Fn(Option<String>, u32) -> O,
    O: Operation,
    O::Output: CursorListing,
{
    pub(crate) fn new(client: C, limit: NonZeroU32, make: F) -> Self {
        Self {
            client,
            make,
            cursor: None,
            seen: HashSet::new(),
            limit,
            done: false,
            _op: PhantomData,
        }
    }

    fn advance(
        &mut self,
        page: CursorPage<<O::Output as CursorListing>::Item>,
    ) -> Result<Vec<<O::Output as CursorListing>::Item>> {
        if !page.has_more {
            self.done = true;
            return Ok(page.items);
        }
        let next = page.next_cursor.ok_or_else(|| {
            self.done = true;
            Error::Decode("invalid cursor pagination metadata: missing next cursor".to_owned())
        })?;
        if !self.seen.insert(next.clone()) {
            self.done = true;
            return Err(Error::Decode(
                "invalid cursor pagination metadata: repeated next cursor".to_owned(),
            ));
        }
        self.cursor = Some(next);
        Ok(page.items)
    }
}

#[cfg(feature = "async")]
impl<'a, F, O> CursorPaginator<crate::v4::BlooioAccount<'a>, F, O>
where
    F: Fn(Option<String>, u32) -> O,
    O: Operation,
    O::Output: CursorListing,
{
    /// Fetch the next page.
    pub async fn next_page(&mut self) -> Option<Result<Vec<<O::Output as CursorListing>::Item>>> {
        if self.done {
            return None;
        }
        let op = (self.make)(self.cursor.clone(), self.limit.get());
        match self.client.send(op).await {
            Ok(output) => Some(self.advance(output.into_cursor_page())),
            Err(error) => {
                self.done = true;
                Some(Err(error))
            }
        }
    }
    /// Collect all remaining items.
    pub async fn collect_all(mut self) -> Result<Vec<<O::Output as CursorListing>::Item>> {
        let mut all = Vec::new();
        while let Some(page) = self.next_page().await {
            all.extend(page?);
        }
        Ok(all)
    }
    /// Stream remaining items across page boundaries.
    pub fn stream(
        self,
    ) -> impl futures_core::Stream<Item = Result<<O::Output as CursorListing>::Item>> + 'a
    where
        F: 'a,
        O: 'a,
    {
        async_stream::stream! {
            let mut paginator = self;
            while let Some(page) = paginator.next_page().await {
                match page {
                    Ok(items) => for item in items { yield Ok(item); },
                    Err(error) => { yield Err(error); break; }
                }
            }
        }
    }
}

#[cfg(feature = "sync")]
impl<F, O> CursorPaginator<crate::v4::BlockingBlooioAccount<'_>, F, O>
where
    F: Fn(Option<String>, u32) -> O,
    O: Operation,
    O::Output: CursorListing,
{
    /// Fetch the next page.
    pub fn next_page(&mut self) -> Option<Result<Vec<<O::Output as CursorListing>::Item>>> {
        if self.done {
            return None;
        }
        let op = (self.make)(self.cursor.clone(), self.limit.get());
        match self.client.send(op) {
            Ok(output) => Some(self.advance(output.into_cursor_page())),
            Err(error) => {
                self.done = true;
                Some(Err(error))
            }
        }
    }
    /// Collect all remaining items.
    pub fn collect_all(mut self) -> Result<Vec<<O::Output as CursorListing>::Item>> {
        let mut all = Vec::new();
        while let Some(page) = self.next_page() {
            all.extend(page?);
        }
        Ok(all)
    }
}

#[cfg(feature = "sync")]
impl<F, O> Iterator for CursorPaginator<crate::v4::BlockingBlooioAccount<'_>, F, O>
where
    F: Fn(Option<String>, u32) -> O,
    O: Operation,
    O::Output: CursorListing,
{
    type Item = Result<Vec<<O::Output as CursorListing>::Item>>;
    fn next(&mut self) -> Option<Self::Item> {
        self.next_page()
    }
}
