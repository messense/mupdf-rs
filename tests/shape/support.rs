use std::path::Path;

use mupdf::pdf::PdfPage;
use mupdf::{Colorspace, Image, ImageFormat, Matrix, Pixmap};

const INK_THRESHOLD: u8 = 250;
const MAX_DIFFERENT_SAMPLE_RATIO: f64 = 0.025;
const MAX_MEAN_ABS_DIFF: f64 = 0.5;
const MAX_SAMPLE_ABS_DIFF: u8 = 160;
const MAX_INK_COUNT_RATIO_DELTA: f64 = 0.15;
const MAX_INK_BBOX_DELTA: u32 = 4;
const MIN_INK_JACCARD: f64 = 0.89;

#[derive(Clone, Copy, Debug)]
struct InkBox {
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
}

#[derive(Clone, Copy, Debug)]
struct InkStats {
    count: usize,
    bbox: Option<InkBox>,
}

#[derive(Clone, Copy, Debug)]
struct DiffStats {
    different_sample_ratio: f64,
    mean_abs_diff: f64,
    max_abs_diff: u8,
    actual_ink: InkStats,
    expected_ink: InkStats,
    ink_count_ratio_delta: f64,
    ink_bbox_delta: u32,
    ink_jaccard: f64,
}

pub fn render_page(page: &PdfPage) -> Pixmap {
    page.to_pixmap(
        &Matrix::new_scale(1.0, 1.0),
        &Colorspace::device_rgb(),
        false,
        true,
    )
    .unwrap()
}

pub fn assert_snapshot(snapshot: &str, rendered: &Pixmap) {
    if std::env::var_os("UPDATE_SHAPE_SNAPSHOTS").is_some() {
        rendered.save_as(snapshot, ImageFormat::PNG).unwrap();
    }

    assert!(
        Path::new(snapshot).exists(),
        "missing snapshot {snapshot}; rerun with UPDATE_SHAPE_SNAPSHOTS=1"
    );
    let expected = Image::from_file(snapshot).unwrap().to_pixmap().unwrap();
    assert_same_layout(snapshot, rendered, &expected);

    if rendered.samples() == expected.samples() {
        return;
    }

    if cfg!(target_os = "macos") {
        panic!("snapshot {snapshot} differs from the checked-in PNG");
    }

    let stats = diff_stats(rendered, &expected);
    assert!(
        stats.different_sample_ratio <= MAX_DIFFERENT_SAMPLE_RATIO,
        "snapshot {snapshot} differs in too many samples: {stats:?}"
    );
    assert!(
        stats.mean_abs_diff <= MAX_MEAN_ABS_DIFF,
        "snapshot {snapshot} mean absolute diff is too large: {stats:?}"
    );
    assert!(
        stats.max_abs_diff <= MAX_SAMPLE_ABS_DIFF,
        "snapshot {snapshot} has an unexpectedly large sample diff: {stats:?}"
    );
    assert!(
        stats.actual_ink.count > 0,
        "snapshot {snapshot} rendered no visible ink: {stats:?}"
    );
    assert!(
        stats.expected_ink.count > 0,
        "snapshot {snapshot} has no visible ink in the checked-in PNG: {stats:?}"
    );
    assert!(
        stats.ink_count_ratio_delta <= MAX_INK_COUNT_RATIO_DELTA,
        "snapshot {snapshot} ink coverage changed too much: {stats:?}"
    );
    assert!(
        stats.ink_bbox_delta <= MAX_INK_BBOX_DELTA,
        "snapshot {snapshot} ink bounds moved too far: {stats:?}"
    );
    assert!(
        stats.ink_jaccard >= MIN_INK_JACCARD,
        "snapshot {snapshot} ink mask changed too much: {stats:?}"
    );
}

fn assert_same_layout(snapshot: &str, actual: &Pixmap, expected: &Pixmap) {
    assert_eq!(
        actual.width(),
        expected.width(),
        "snapshot {snapshot} width"
    );
    assert_eq!(
        actual.height(),
        expected.height(),
        "snapshot {snapshot} height"
    );
    assert_eq!(actual.n(), expected.n(), "snapshot {snapshot} channels");
    assert_eq!(
        actual.samples().len(),
        expected.samples().len(),
        "snapshot {snapshot} sample count"
    );
}

fn diff_stats(actual: &Pixmap, expected: &Pixmap) -> DiffStats {
    let mut different_samples = 0usize;
    let mut total_abs_diff = 0u64;
    let mut max_abs_diff = 0u8;
    for (actual, expected) in actual.samples().iter().zip(expected.samples()) {
        let diff = actual.abs_diff(*expected);
        if diff != 0 {
            different_samples += 1;
            total_abs_diff += u64::from(diff);
            max_abs_diff = max_abs_diff.max(diff);
        }
    }

    let sample_count = actual.samples().len() as f64;
    let actual_ink = ink_stats(actual);
    let expected_ink = ink_stats(expected);
    let ink_count_ratio_delta = if expected_ink.count == 0 {
        f64::INFINITY
    } else {
        ((actual_ink.count as f64) / (expected_ink.count as f64) - 1.0).abs()
    };
    let ink_bbox_delta = match (actual_ink.bbox, expected_ink.bbox) {
        (Some(actual), Some(expected)) => actual.max_delta(expected),
        (None, None) => 0,
        _ => u32::MAX,
    };

    DiffStats {
        different_sample_ratio: different_samples as f64 / sample_count,
        mean_abs_diff: total_abs_diff as f64 / sample_count,
        max_abs_diff,
        actual_ink,
        expected_ink,
        ink_count_ratio_delta,
        ink_bbox_delta,
        ink_jaccard: ink_jaccard(actual, expected),
    }
}

fn ink_stats(pixmap: &Pixmap) -> InkStats {
    let channels = pixmap.n() as usize;
    let samples = pixmap.samples();
    let mut count = 0usize;
    let mut bbox: Option<InkBox> = None;

    for pixel in 0..(samples.len() / channels) {
        let offset = pixel * channels;
        if !is_ink(&samples[offset..offset + channels]) {
            continue;
        }

        let x = pixel as u32 % pixmap.width();
        let y = pixel as u32 / pixmap.width();
        count += 1;
        match &mut bbox {
            Some(bbox) => bbox.include(x, y),
            None => bbox = Some(InkBox::new(x, y)),
        }
    }

    InkStats { count, bbox }
}

fn ink_jaccard(actual: &Pixmap, expected: &Pixmap) -> f64 {
    let channels = actual.n() as usize;
    let mut intersection = 0usize;
    let mut union = 0usize;

    for pixel in 0..(actual.samples().len() / channels) {
        let offset = pixel * channels;
        let actual_ink = is_ink(&actual.samples()[offset..offset + channels]);
        let expected_ink = is_ink(&expected.samples()[offset..offset + channels]);
        if actual_ink && expected_ink {
            intersection += 1;
        }
        if actual_ink || expected_ink {
            union += 1;
        }
    }

    if union == 0 {
        1.0
    } else {
        intersection as f64 / union as f64
    }
}

fn is_ink(pixel: &[u8]) -> bool {
    pixel.iter().any(|sample| *sample < INK_THRESHOLD)
}

impl InkBox {
    fn new(x: u32, y: u32) -> Self {
        Self {
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
        }
    }

    fn include(&mut self, x: u32, y: u32) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    fn max_delta(self, other: Self) -> u32 {
        self.min_x
            .abs_diff(other.min_x)
            .max(self.min_y.abs_diff(other.min_y))
            .max(self.max_x.abs_diff(other.max_x))
            .max(self.max_y.abs_diff(other.max_y))
    }
}
