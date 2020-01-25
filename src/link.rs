use crate::Rect;

#[derive(Debug, Clone)]
pub struct Link {
    pub bounds: Rect,
    pub page: u32,
    pub uri: String,
}
