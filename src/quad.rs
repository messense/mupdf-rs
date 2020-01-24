#[derive(Debug, Clone, PartialEq)]
pub struct Quad {
    pub ul_x: f32,
    pub ul_y: f32,
    pub ur_x: f32,
    pub ur_y: f32,
    pub ll_x: f32,
    pub ll_y: f32,
    pub lr_x: f32,
    pub lr_y: f32,
}

impl Quad {
    pub fn new(
        ul_x: f32,
        ul_y: f32,
        ur_x: f32,
        ur_y: f32,
        ll_x: f32,
        ll_y: f32,
        lr_x: f32,
        lr_y: f32,
    ) -> Self {
        Self {
            ul_x,
            ul_y,
            ur_x,
            ur_y,
            ll_x,
            ll_y,
            lr_x,
            lr_y,
        }
    }
}
