use mupdf_sys::*;

use crate::{context, Error, Matrix, Rect};

#[derive(Debug)]
pub struct Shade {
    pub(crate) inner: *mut fz_shade,
}

impl Shade {
    /// The bounding box of the shading painted with `ctm` (MuPDF's `fz_bound_shade`), in the
    /// device space of `ctm`. It is the rectangle MuPDF rasterises for the shading, before any
    /// clip: for axial and radial shadings the transformed `/BBox`, for mesh shadings the mesh
    /// bounds intersected with it.
    ///
    /// # Errors
    ///
    /// [`Error::UnexpectedNullPtr`] when the shading holds no MuPDF object.
    pub fn bounds(&self, ctm: &Matrix) -> Result<Rect, Error> {
        if self.inner.is_null() {
            return Err(Error::UnexpectedNullPtr);
        }
        // SAFETY: `inner` is a live `fz_shade` for as long as `self` is; `fz_bound_shade` only
        // reads it. It can throw only through `fz_bound_mesh` on an unexpected mesh type, which
        // the PDF loader rules out for shadings it loads.
        let rect = unsafe { fz_bound_shade(context(), self.inner, ctm.into()) };
        Ok(rect.into())
    }
}

impl Drop for Shade {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                fz_drop_shade(context(), self.inner);
            }
        }
    }
}
