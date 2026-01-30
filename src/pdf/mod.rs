pub mod annotation;
pub mod document;
pub mod filter;
pub mod graft_map;
pub mod intent;
pub mod links;
pub mod object;
pub mod page;

pub use annotation::{LineEndingStyle, PdfAnnotation, PdfAnnotationType};
pub use document::{Encryption, PdfDocument, PdfWriteOptions, Permission};
pub use filter::PdfFilterOptions;
pub use graft_map::PdfGraftMap;
pub use intent::Intent;
pub use links::{
    DestPageResolver, FileSpec, LinkAction, PdfAction, PdfDestination, PdfLink, PdfLinkAnnot,
};
pub use object::PdfObject;
pub use page::PdfPage;

pub struct DocOperation<'a> {
    doc: &'a mut PdfDocument,
    success: bool,
}

impl<'a> DocOperation<'a> {
    fn begin(doc: &'a mut PdfDocument, name: &str) -> Result<Self, crate::Error> {
        doc.begin_operation(name)?;
        Ok(Self {
            doc,
            success: false,
        })
    }

    fn commit(mut self) {
        self.success = true;
    }
}

impl Drop for DocOperation<'_> {
    fn drop(&mut self) {
        if self.success {
            let _ = self.doc.end_operation();
        } else {
            let _ = self.doc.abandon_operation();
        }
    }
}
