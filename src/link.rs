use crate::{document::Location, Rect};

/// A list of interactive links on a page.
#[derive(Debug, Clone)]
pub struct Link {
    pub bounds: Rect,
    pub location: Option<Location>,
    pub uri: String,
}
