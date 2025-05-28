use crate::document::Location;

/// a tree of the outline of a document (also known as table of contents).
#[derive(Debug)]
pub struct Outline {
    pub title: String,
    pub uri: Option<String>,
    pub location: Option<Location>,
    pub down: Vec<Outline>,
}
