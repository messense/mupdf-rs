use crate::link::LinkDestination;

/// a tree of the outline of a document (also known as table of contents).
#[derive(Debug)]
pub struct Outline {
    pub title: String,
    pub uri: Option<String>,
    pub dest: Option<LinkDestination>,
    pub down: Vec<Outline>,
}
