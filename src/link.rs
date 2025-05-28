use std::ffi::CStr;

use crate::{context, document::Location, DestinationKind, Document, Error, Rect};

use mupdf_sys::*;

/// A list of interactive links on a page.
#[derive(Debug, Clone)]
pub struct Link {
    pub bounds: Rect,
    pub dest: Option<LinkDestination>,
    pub uri: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinkDestination {
    pub loc: Location,
    pub kind: DestinationKind,
}

impl LinkDestination {
    pub(crate) fn from_uri(doc: &Document, uri: &CStr) -> Result<Option<Self>, Error> {
        let external = unsafe { fz_is_external_link(context(), uri.as_ptr()) } != 0;
        if external {
            return Ok(None);
        }

        let dest =
            unsafe { ffi_try!(mupdf_resolve_link_dest(context(), doc.inner, uri.as_ptr())) }?;
        if dest.loc.page < 0 {
            return Ok(None);
        }

        let page_number = unsafe { fz_page_number_from_location(context(), doc.inner, dest.loc) };

        Ok(Some(Self {
            loc: Location {
                chapter: dest.loc.chapter as u32,
                page: dest.loc.page as u32,
                page_number: page_number as u32,
            },
            kind: DestinationKind::from_link_dest(dest),
        }))
    }
}
