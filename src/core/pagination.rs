//! Offset/limit pagination helpers.

use std::marker::PhantomData;
use std::num::NonZeroU32;

use serde::Deserialize;

use crate::core::operation::Operation;
use crate::error::Result;

/// The `Pagination` object carried by list responses.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Pagination {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub total: Option<i64>,
    /// Number of items returned in this page (some endpoints report it).
    pub returned: Option<i64>,
    /// Whether further pages exist (some endpoints report it).
    pub has_more: Option<bool>,
}

/// A single page of results plus its pagination metadata.
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub pagination: Option<Pagination>,
}

/// Implemented by list-operation output types so the generic paginators can
/// extract the items and pagination cursor uniformly.
pub trait Listing {
    /// The element type of the list.
    type Item;
    /// Consume the response into a [`Page`].
    fn into_page(self) -> Page<Self::Item>;
}

/// Default page size used by the `*_all` paginator helpers (50).
pub const DEFAULT_PAGE_SIZE: NonZeroU32 = match NonZeroU32::new(50) {
    Some(n) => n,
    None => unreachable!(),
};

/// Shared offset-stepping state and stop logic for both paginators.
struct Cursor {
    offset: u32,
    limit: NonZeroU32,
    done: bool,
}

impl Cursor {
    fn new(limit: NonZeroU32) -> Self {
        Cursor {
            offset: 0,
            limit,
            done: false,
        }
    }

    /// Apply a fetched page and return its items, updating the cursor.
    fn advance<T>(&mut self, page: Page<T>) -> Vec<T> {
        let got = u32::try_from(page.items.len()).unwrap_or(u32::MAX);
        self.offset += got;
        let reached_total = page
            .pagination
            .as_ref()
            .and_then(|p| p.total)
            .is_some_and(|total| i64::from(self.offset) >= total);
        if got < self.limit.get() || got == 0 || reached_total {
            self.done = true;
        }
        page.items
    }
}

/// A lazy paginator that fetches successive offset pages on demand.
///
/// `make` builds the list operation for a given `(offset, limit)`.
pub struct Paginator<'c, C, F, O>
where
    F: Fn(u32, u32) -> O,
    O: Operation,
    O::Output: Listing,
{
    client: &'c C,
    make: F,
    cursor: Cursor,
    _op: PhantomData<O>,
}

impl<C, F, O> core::fmt::Debug for Paginator<'_, C, F, O>
where
    F: Fn(u32, u32) -> O,
    O: Operation,
    O::Output: Listing,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Paginator")
            .field("cursor", &self.cursor.offset)
            .finish_non_exhaustive()
    }
}

impl<'c, C, F, O> Paginator<'c, C, F, O>
where
    F: Fn(u32, u32) -> O,
    O: Operation,
    O::Output: Listing,
{
    pub(crate) fn new(client: &'c C, limit: NonZeroU32, make: F) -> Self {
        Paginator {
            client,
            make,
            cursor: Cursor::new(limit),
            _op: PhantomData,
        }
    }
}

#[cfg(feature = "async")]
impl<F, O> Paginator<'_, crate::Client, F, O>
where
    F: Fn(u32, u32) -> O,
    O: Operation,
    O::Output: Listing,
{
    /// Fetch the next page, or `None` once exhausted.
    pub async fn next_page(&mut self) -> Option<Result<Vec<<O::Output as Listing>::Item>>> {
        if self.cursor.done {
            return None;
        }
        let op = (self.make)(self.cursor.offset, self.cursor.limit.get());
        match self.client.send(op).await {
            Ok(out) => Some(Ok(self.cursor.advance(out.into_page()))),
            Err(e) => {
                self.cursor.done = true;
                Some(Err(e))
            }
        }
    }

    /// Drain every remaining page into a single `Vec`.
    pub async fn collect_all(mut self) -> Result<Vec<<O::Output as Listing>::Item>> {
        let mut all = Vec::new();
        while let Some(page) = self.next_page().await {
            all.extend(page?);
        }
        Ok(all)
    }
}

#[cfg(feature = "sync")]
impl<F, O> Paginator<'_, crate::BlockingClient, F, O>
where
    F: Fn(u32, u32) -> O,
    O: Operation,
    O::Output: Listing,
{
    /// Fetch the next page, or `None` once exhausted.
    pub fn next_page(&mut self) -> Option<Result<Vec<<O::Output as Listing>::Item>>> {
        if self.cursor.done {
            return None;
        }
        let op = (self.make)(self.cursor.offset, self.cursor.limit.get());
        match self.client.send(op) {
            Ok(out) => Some(Ok(self.cursor.advance(out.into_page()))),
            Err(e) => {
                self.cursor.done = true;
                Some(Err(e))
            }
        }
    }

    /// Drain every remaining page into a single `Vec`.
    pub fn collect_all(mut self) -> Result<Vec<<O::Output as Listing>::Item>> {
        let mut all = Vec::new();
        while let Some(page) = self.next_page() {
            all.extend(page?);
        }
        Ok(all)
    }
}

#[cfg(feature = "sync")]
impl<F, O> Iterator for Paginator<'_, crate::BlockingClient, F, O>
where
    F: Fn(u32, u32) -> O,
    O: Operation,
    O::Output: Listing,
{
    type Item = Result<Vec<<O::Output as Listing>::Item>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_page()
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::unreadable_literal
)]
mod tests {
    use super::*;

    /// Build a `NonZeroU32` page size for tests.
    fn nz(n: u32) -> NonZeroU32 {
        NonZeroU32::new(n).unwrap()
    }

    #[test]
    fn cursor_stops_on_short_page() {
        let mut c = Cursor::new(nz(50));
        let page = Page {
            items: vec![1, 2, 3],
            pagination: Some(Pagination {
                limit: Some(50),
                offset: Some(0),
                total: Some(3),
                ..Default::default()
            }),
        };
        let items = c.advance(page);
        assert_eq!(items, vec![1, 2, 3]);
        assert_eq!(c.offset, 3);
        assert!(c.done);
    }

    #[test]
    fn cursor_continues_on_full_page() {
        let mut c = Cursor::new(nz(2));
        let page = Page {
            items: vec![1, 2],
            pagination: Some(Pagination {
                limit: Some(2),
                offset: Some(0),
                total: Some(10),
                ..Default::default()
            }),
        };
        c.advance(page);
        assert_eq!(c.offset, 2);
        assert!(!c.done);
    }

    #[test]
    fn cursor_stops_when_total_reached_even_on_full_page() {
        let mut c = Cursor::new(nz(2));
        let page = Page {
            items: vec![1, 2],
            pagination: Some(Pagination {
                limit: Some(2),
                offset: Some(0),
                total: Some(2),
                ..Default::default()
            }),
        };
        c.advance(page);
        assert!(c.done);
    }

    #[test]
    fn cursor_continues_on_full_page_without_pagination() {
        // No `pagination` metadata: a full page must keep the cursor live so the
        // next page is fetched. (Termination then relies on a later short/empty
        // page, exercised by the two tests below.)
        let mut c = Cursor::new(nz(2));
        let page = Page {
            items: vec![1, 2],
            pagination: None,
        };
        c.advance(page);
        assert_eq!(c.offset, 2);
        assert!(!c.done);
    }

    #[test]
    fn cursor_stops_on_short_page_without_pagination() {
        // A short page (got < limit) is the terminator when no `total` is given.
        let mut c = Cursor::new(nz(50));
        let page = Page {
            items: vec![1, 2, 3],
            pagination: None,
        };
        c.advance(page);
        assert_eq!(c.offset, 3);
        assert!(c.done);
    }

    #[test]
    fn cursor_stops_on_empty_page() {
        // An empty page always terminates, even at a full-limit offset boundary
        // with no pagination metadata. This is the guard against runaway loops.
        let mut c = Cursor::new(nz(2));
        let page: Page<i32> = Page {
            items: vec![],
            pagination: None,
        };
        let items = c.advance(page);
        assert!(items.is_empty());
        assert_eq!(c.offset, 0);
        assert!(c.done);
    }
}
