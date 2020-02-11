#[derive(Debug)]
pub struct Outline {
    pub title: String,
    pub uri: Option<String>,
    pub page: Option<u32>,
    pub down: Vec<Outline>,
    pub x: f32,
    pub y: f32,
}
