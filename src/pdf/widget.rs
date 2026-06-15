use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::ptr::NonNull;

use mupdf_sys::*;

use crate::pdf::{PdfAnnotation, PdfDocument};
use crate::{context, from_enum, Error};

from_enum! { pdf_widget_type => pdf_widget_type,
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum WidgetType {
        Unknown = PDF_WIDGET_TYPE_UNKNOWN,
        Button = PDF_WIDGET_TYPE_BUTTON,
        Checkbox = PDF_WIDGET_TYPE_CHECKBOX,
        Combobox = PDF_WIDGET_TYPE_COMBOBOX,
        Listbox = PDF_WIDGET_TYPE_LISTBOX,
        RadioButton = PDF_WIDGET_TYPE_RADIOBUTTON,
        Signature = PDF_WIDGET_TYPE_SIGNATURE,
        Text = PDF_WIDGET_TYPE_TEXT,
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct FieldFlags: i32 {
        const READ_ONLY = PDF_FIELD_IS_READ_ONLY as i32;
        const REQUIRED = PDF_FIELD_IS_REQUIRED as i32;
        const NO_EXPORT = PDF_FIELD_IS_NO_EXPORT as i32;
        const MULTILINE = PDF_TX_FIELD_IS_MULTILINE as i32;
        const PASSWORD = PDF_TX_FIELD_IS_PASSWORD as i32;
        const FILE_SELECT = PDF_TX_FIELD_IS_FILE_SELECT as i32;
        const DO_NOT_SPELL_CHECK = PDF_TX_FIELD_IS_DO_NOT_SPELL_CHECK as i32;
        const DO_NOT_SCROLL = PDF_TX_FIELD_IS_DO_NOT_SCROLL as i32;
        const COMB = PDF_TX_FIELD_IS_COMB as i32;
        const RICH_TEXT = PDF_TX_FIELD_IS_RICH_TEXT as i32;
        const NO_TOGGLE_TO_OFF = PDF_BTN_FIELD_IS_NO_TOGGLE_TO_OFF as i32;
        const RADIO = PDF_BTN_FIELD_IS_RADIO as i32;
        const PUSHBUTTON = PDF_BTN_FIELD_IS_PUSHBUTTON as i32;
        const RADIOS_IN_UNISON = PDF_BTN_FIELD_IS_RADIOS_IN_UNISON as i32;
        const COMBO = PDF_CH_FIELD_IS_COMBO as i32;
        const EDIT = PDF_CH_FIELD_IS_EDIT as i32;
        const SORT = PDF_CH_FIELD_IS_SORT as i32;
        const MULTI_SELECT = PDF_CH_FIELD_IS_MULTI_SELECT as i32;
        const COMMIT_ON_SEL_CHANGE = PDF_CH_FIELD_IS_COMMIT_ON_SEL_CHANGE as i32;
    }
}

#[derive(Debug)]
pub struct PdfWidget {
    annot: PdfAnnotation,
}

impl PdfWidget {
    pub(crate) unsafe fn from_raw(ptr: *mut pdf_annot) -> Self {
        Self {
            annot: PdfAnnotation::from_raw(ptr),
        }
    }

    pub(crate) unsafe fn from_raw_keep_ref(ptr: *mut pdf_annot) -> Self {
        Self {
            annot: PdfAnnotation::from_raw_keep_ref(ptr),
        }
    }

    pub fn annotation(&self) -> &PdfAnnotation {
        &self.annot
    }

    pub fn into_annotation(self) -> PdfAnnotation {
        self.annot
    }

    pub fn xref(&self) -> Result<i32, Error> {
        self.annot.xref()
    }

    fn assert_document_owner(&self, doc: &PdfDocument) -> Result<(), Error> {
        let page = self.annot.page_ptr()?;
        assert_eq!(
            doc.as_raw(),
            unsafe { (*page).doc },
            "PdfWidget ownership mismatch: the widget is not attached to the provided PdfDocument"
        );
        Ok(())
    }

    pub fn r#type(&self) -> Result<WidgetType, Error> {
        self.annot.ensure_attached()?;
        unsafe { ffi_try!(mupdf_pdf_widget_type(context(), self.annot.inner.as_ptr())) }
            .map(|kind| WidgetType::try_from(kind).unwrap_or(WidgetType::Unknown))
    }

    pub fn update(&mut self) -> Result<bool, Error> {
        self.annot.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_update_widget(
                context(),
                self.annot.inner.as_ptr()
            ))
        }
        .map(|updated| updated != 0)
    }

    pub fn is_signed(&self) -> Result<bool, Error> {
        self.annot.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_widget_is_signed(
                context(),
                self.annot.inner.as_ptr()
            ))
        }
        .map(|signed| signed != 0)
    }

    pub fn is_readonly(&self) -> Result<bool, Error> {
        self.annot.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_widget_is_readonly(
                context(),
                self.annot.inner.as_ptr()
            ))
        }
        .map(|readonly| readonly != 0)
    }

    pub fn name(&self) -> Result<Option<String>, Error> {
        self.annot.ensure_attached()?;
        let ptr = unsafe {
            ffi_try!(mupdf_pdf_load_widget_field_name(
                context(),
                self.annot.inner.as_ptr()
            ))?
        };
        if ptr.is_null() {
            return Ok(None);
        }
        let value = unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned();
        unsafe { fz_free(context(), ptr.cast()) };
        Ok(Some(value))
    }

    pub fn field_type_name(&self) -> Result<Option<String>, Error> {
        self.annot.ensure_attached()?;
        let ptr = unsafe {
            ffi_try!(mupdf_pdf_widget_field_type_string(
                context(),
                self.annot.inner.as_ptr()
            ))?
        };
        if ptr.is_null() {
            return Ok(None);
        }
        Ok(Some(
            unsafe { CStr::from_ptr(ptr) }
                .to_string_lossy()
                .into_owned(),
        ))
    }

    pub fn field_flags(&self) -> Result<FieldFlags, Error> {
        self.annot.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_widget_field_flags(
                context(),
                self.annot.inner.as_ptr()
            ))
        }
        .map(FieldFlags::from_bits_truncate)
    }

    pub fn value(&self) -> Result<Option<String>, Error> {
        self.annot.ensure_attached()?;
        let ptr = unsafe {
            ffi_try!(mupdf_pdf_widget_field_value(
                context(),
                self.annot.inner.as_ptr()
            ))?
        };
        if ptr.is_null() {
            return Ok(None);
        }
        Ok(Some(
            unsafe { CStr::from_ptr(ptr) }
                .to_string_lossy()
                .into_owned(),
        ))
    }

    pub fn label(&self) -> Result<Option<String>, Error> {
        self.annot.ensure_attached()?;
        let ptr = unsafe {
            ffi_try!(mupdf_pdf_widget_field_label(
                context(),
                self.annot.inner.as_ptr()
            ))?
        };
        if ptr.is_null() {
            return Ok(None);
        }
        Ok(Some(
            unsafe { CStr::from_ptr(ptr) }
                .to_string_lossy()
                .into_owned(),
        ))
    }

    pub fn set_value(
        &mut self,
        doc: &mut PdfDocument,
        value: &str,
        ignore_trigger_events: bool,
    ) -> Result<bool, Error> {
        self.annot.ensure_attached()?;
        self.assert_document_owner(doc)?;
        let value = CString::new(value)?;
        unsafe {
            ffi_try!(mupdf_pdf_set_widget_field_value(
                context(),
                doc.as_raw(),
                self.annot.inner.as_ptr(),
                value.as_ptr(),
                i32::from(ignore_trigger_events)
            ))
        }
        .map(|accepted| accepted != 0)
    }

    pub fn reset(&mut self, doc: &mut PdfDocument) -> Result<(), Error> {
        self.annot.ensure_attached()?;
        self.assert_document_owner(doc)?;
        unsafe {
            ffi_try!(mupdf_pdf_reset_widget_field(
                context(),
                doc.as_raw(),
                self.annot.inner.as_ptr()
            ))
        }
    }
}

#[derive(Debug)]
pub struct PdfWidgetIter<'a> {
    pub(crate) next: Option<NonNull<pdf_annot>>,
    pub(crate) marker: PhantomData<&'a PdfWidget>,
}

impl Iterator for PdfWidgetIter<'_> {
    type Item = PdfWidget;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.next?;
        let next = unsafe { pdf_next_widget(context(), current.as_ptr()) };
        self.next = NonNull::new(next);
        Some(unsafe { PdfWidget::from_raw_keep_ref(current.as_ptr()) })
    }
}
