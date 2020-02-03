use std::fmt;

use crate::Rect;

#[derive(Debug, Clone)]
pub struct Link {
    pub bounds: Rect,
    pub page: u32,
    pub uri: String,
}

impl fmt::Display for Link {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Link(b={},page={},uri={})",
            self.bounds, self.page, self.uri
        )
    }
}
