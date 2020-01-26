use crate::{ColorSpace, Error, Image, Matrix, Path, Rect, Shade, StrokeState, Text};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlendMode {
    /* PDF 1.4 -- standard separable */
    Normal = 0,
    Multiply = 1,
    Screen = 2,
    Overlay = 3,
    Darken = 4,
    Lighten = 5,
    ColorDodge = 6,
    ColorBurn = 7,
    HardLight = 8,
    SoftLight = 9,
    Difference = 10,
    Exclusion = 11,
    /* PDF 1.4 -- standard non-separable */
    Hue = 12,
    Saturation = 13,
    Color = 14,
    Luminosity = 15,
}

pub trait Device {
    fn close(&mut self) -> Result<(), Error>;
    fn fill_path(
        &self,
        path: &Path,
        even_odd: bool,
        ctm: &Matrix,
        cs: &ColorSpace,
        color: &[f32],
        alpha: f32,
        cp: i32,
    ) -> Result<(), Error>;
    fn stroke_path(
        &self,
        path: &Path,
        stroke: &StrokeState,
        ctm: &Matrix,
        cs: &ColorSpace,
        color: &[f32],
        alpha: f32,
        cp: i32,
    ) -> Result<(), Error>;
    fn clip_path(&self, path: &Path, even_odd: bool, ctm: &Matrix) -> Result<(), Error>;
    fn clip_stroke_path(&self, path: &Path, stoke: &StrokeState, ctm: &Matrix)
        -> Result<(), Error>;
    fn fill_text(
        &self,
        text: &Text,
        ctm: &Matrix,
        cs: &ColorSpace,
        color: &[f32],
        alpha: f32,
        cp: i32,
    ) -> Result<(), Error>;
    fn stroke_text(
        &self,
        text: &Text,
        stroke: &StrokeState,
        ctm: &Matrix,
        cs: &ColorSpace,
        color: &[f32],
        alpha: f32,
        cp: i32,
    ) -> Result<(), Error>;
    fn clip_text(&self, text: &Text, ctm: &Matrix) -> Result<(), Error>;
    fn clip_stroke_text(
        &self,
        text: &Text,
        stroke: &StrokeState,
        ctm: &Matrix,
    ) -> Result<(), Error>;
    fn ignore_text(&self, text: &Text, ctm: &Matrix) -> Result<(), Error>;
    fn fill_shade(&self, shd: &Shade, ctm: &Matrix, alpha: f32, cp: i32) -> Result<(), Error>;
    fn fill_image(&self, image: &Image, ctm: &Matrix, alpha: f32, cp: i32) -> Result<(), Error>;
    fn fill_image_mask(
        &self,
        image: &Image,
        ctm: &Matrix,
        cs: &ColorSpace,
        color: &[f32],
        alpha: f32,
        cp: i32,
    ) -> Result<(), Error>;
    fn clip_image_mask(&self, image: &Image, ctm: &Matrix) -> Result<(), Error>;
    fn pop_clip(&self) -> Result<(), Error>;
    fn begin_mask(
        &self,
        area: Rect,
        luminosity: bool,
        cs: &ColorSpace,
        bc: &[f32],
        cp: i32,
    ) -> Result<(), Error>;
    fn end_mask(&self) -> Result<(), Error>;
    fn begin_group(
        &self,
        area: Rect,
        cs: &ColorSpace,
        isolated: bool,
        knockout: bool,
        blend_mode: BlendMode,
        alpha: f32,
    ) -> Result<(), Error>;
    fn end_group(&self) -> Result<(), Error>;
    fn begin_tile(
        &self,
        area: Rect,
        view: Rect,
        xstep: f32,
        ystep: f32,
        ctm: &Matrix,
        id: i32,
    ) -> Result<i32, Error>;
    fn end_tile(&self) -> Result<(), Error>;
    fn begin_layer(&self, name: &str) -> Result<(), Error>;
    fn end_layer(&self) -> Result<(), Error>;
}
