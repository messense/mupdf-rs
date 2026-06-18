# mupdf-rs

[![GitHub Actions](https://github.com/messense/mupdf-rs/workflows/CI/badge.svg)](https://github.com/messense/mupdf-rs/actions?query=workflow%3ACI)
[![Crates.io](https://img.shields.io/crates/v/mupdf.svg)](https://crates.io/crates/mupdf)
[![docs.rs](https://docs.rs/mupdf/badge.svg)](https://docs.rs/mupdf/)

Safe Rust bindings to [MuPDF](https://github.com/ArtifexSoftware/mupdf) for reading,
rendering, extracting, creating, and editing documents.

The `mupdf` crate provides the high-level safe API, while `mupdf-sys` builds MuPDF and
exposes the low-level FFI layer.

## What you can do with it

- Open documents from files or bytes, authenticate encrypted files, inspect metadata,
  outlines, permissions, output intents, and page counts.
- Render pages to pixmaps, SVG, display lists, or custom MuPDF devices.
- Extract plain text, words with bounding boxes, structured text, images, and vector
  drawings from pages.
- Create and edit PDFs: add pages, insert or delete content streams, insert images and
  fonts, merge/copy/reorder/select pages, save with MuPDF PDF write options, and write
  documents to any `Write` target.
- Work with PDF-specific features such as annotations, widgets/forms, link annotations,
  redactions, page labels, embedded files, optional content groups, outlines, objects, and
  xref streams.
- Draw vector primitives and text on PDF pages with the safe `shape` API.
- Use system fonts or optional bundled Droid/Noto/SIL runtime font providers for fallback
  font resolution.

## Installation

```toml
[dependencies]
mupdf = "0.8"
```

The default feature set builds PDF support along with JavaScript, XPS, SVG, CBZ,
image, HTML, EPUB, system-fonts, Tesseract OCR, Brotli, DOCX output, and Base14 font
support. Optional feature flags include:

- `serde` for `Serialize`/`Deserialize` support on selected types.
- `bundled-fonts-noto`, `bundled-fonts-droid`, `bundled-fonts-sil`, or `bundled-fonts`
  for runtime bundled font providers.
- `zxingcpp` and `libarchive` for the corresponding MuPDF integrations.
- `sys-lib` and `sys-lib-*` flags to use system copies of supported third-party C/C++
  libraries instead of MuPDF's vendored versions.

For smaller builds, disable default features and opt back into only what you need:

```toml
[dependencies]
mupdf = { version = "0.8", default-features = false, features = ["base14-fonts"] }
```

## Quick start

Open a document, extract text from the first page, and render it to PNG:

```rust,no_run
use mupdf::{Colorspace, Document, ImageFormat, Matrix, TextExtractOptions};

fn main() -> Result<(), mupdf::Error> {
    let document = Document::open("input.pdf")?;
    println!("pages: {}", document.page_count()?);

    let page = document.load_page(0)?;
    println!("{}", page.text(TextExtractOptions::default())?);

    let pixmap = page.to_pixmap(
        &Matrix::new_scale(2.0, 2.0),
        &Colorspace::device_rgb(),
        false,
        true,
    )?;
    pixmap.save_as("page.png", ImageFormat::PNG)?;

    Ok(())
}
```

Useful examples:

```console
cargo run --example extract_text -- path/to/document.pdf
cargo run --example extract_images -- path/to/document.pdf
cargo run --example list_annotations -- path/to/document.pdf
cargo run --example shape_demo -- target/shape_demo
```

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

## Building from source

Source checkouts need the MuPDF submodule:

```console
git submodule update --init --recursive
```

Building `mupdf-sys` requires a C/C++ toolchain and libclang for bindgen. With the default
`system-fonts` feature enabled on Linux, install the Fontconfig development package as well
(for example, `libfontconfig1-dev` on Debian/Ubuntu).

## References

1. [MuPDF Explored](https://ghostscript.com/~robin/mupdf_explored.pdf)

## License

This work is released under the AGPL-3.0 license. A copy of the license is provided in the [LICENSE](./LICENSE) file.
