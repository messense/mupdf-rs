#[derive(Debug)]
pub struct Outline {
    pub title: String,
    pub uri: String,
    pub page: i32,
    pub down: Vec<Outline>,
    pub x: f32,
    pub y: f32,
}
