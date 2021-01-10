use crate::pdf::PdfAnnotation;

#[derive(Debug)]
pub enum PdfWidget {
    Button { inner: PdfWidgetInner },
    CheckBox { inner: PdfWidgetInner },
    ComboBox { inner: PdfWidgetInner },
    ListBox { inner: PdfWidgetInner },
    RadioButton { inner: PdfWidgetInner },
    Signature { inner: PdfWidgetInner },
    Text { inner: PdfWidgetInner },
    Unknown { inner: PdfWidgetInner },
}

#[derive(Debug)]
pub struct PdfWidgetInner {
    annot: PdfAnnotation,
}

impl PdfWidget {
    pub fn type_code(&self) -> i32 {
        use PdfWidget::*;

        match *self {
            Unknown { .. } => 0,
            Button { .. } => 1,
            CheckBox { .. } => 2,
            ComboBox { .. } => 3,
            ListBox { .. } => 4,
            RadioButton { .. } => 5,
            Signature { .. } => 6,
            Text { .. } => 7,
        }
    }
}
