# mupdf-rs

[![GitHub Actions](https://github.com/messense/mupdf-rs/workflows/CI/badge.svg)](https://github.com/messense/mupdf-rs/actions?query=workflow%3ACI)
[![Crates.io](https://img.shields.io/crates/v/mupdf.svg)](https://crates.io/crates/mupdf)
[![docs.rs](https://docs.rs/mupdf/badge.svg)](https://docs.rs/mupdf/)

Rust binding to [mupdf](https://github.com/ArtifexSoftware/mupdf)

**Working in progress**

## Shape: Drawing & Text on PDF Pages

`Shape` is a safe, idiomatic Rust port of [PyMuPDF's `Shape` class](https://pymupdf.readthedocs.io/en/latest/shape/).
It provides a builder-style API for accumulating drawing and text operations on a `PdfPage`,
then committing them to the document in a single transaction.

```rust,no_run
use mupdf::pdf::PdfDocument;
use mupdf::shape::{FinishOptions, PdfColor, Shape, TextOptions};
use mupdf::{Point, Rect, Size};

fn main() -> Result<(), mupdf::Error> {
    let mut doc = PdfDocument::new();
    let mut page = doc.new_page(Size::A4)?;
    let mut shape = Shape::new(&mut page)?;
    shape
        .draw_rect(&Rect::new(72.0, 72.0, 272.0, 172.0))?
        .draw_circle(Point::new(372.0, 122.0), 50.0)?
        .finish(&FinishOptions {
            color: Some(PdfColor::rgb(0.0, 0.0, 0.0)),
            fill: Some(PdfColor::rgb(0.9, 0.95, 1.0)),
            width: 1.5,
            ..Default::default()
        })?
        .insert_text(Point::new(72.0, 220.0), "Hello, Shape!", &TextOptions::default())?
        .commit(&mut doc, true)?;
    doc.save("hello_shape.pdf")?;
    Ok(())
}
```

See [`examples/shape_demo.rs`](./examples/shape_demo.rs) for a full kitchen-sink walkthrough
covering primitives, Bezier curves, text boxes, optional content, custom fonts, and more.
For background on the original API, refer to the
[PyMuPDF Shape documentation](https://pymupdf.readthedocs.io/en/latest/shape/).

## Bundled fonts

Large non-URW fonts are split out of `mupdf-sys` and can be loaded at runtime
through optional safe-crate features:

- `bundled-fonts-noto`
- `bundled-fonts-droid`
- `bundled-fonts-sil`
- `bundled-fonts`, or the compatibility alias `all-fonts`, to enable all of them

These features install MuPDF system-font hooks on non-Android, non-wasm targets
so fallback font loading can resolve the bundled providers. Direct font loading,
such as `Font::new("Noto Sans")`, can still resolve bundled fonts by name when the
corresponding feature is enabled.

Published font crates contain regular font files. In this source repository, the
font crate payloads are symlinked to the MuPDF submodule resources to avoid
duplicating the files in git, so source checkouts need initialized submodules and
symlink-capable Git checkout settings.

## References

1. [MuPDF Explored](https://ghostscript.com/~robin/mupdf_explored.pdf)

## License

This work is released under the AGPL-3.0 license. A copy of the license is provided in the [LICENSE](./LICENSE) file.
