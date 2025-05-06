use std::env::{self, current_dir};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::{fs, result};

mod docs;
use docs::DocsCallbacks;

#[cfg(not(target_env = "msvc"))]
mod make;
#[cfg(not(target_env = "msvc"))]
use make::Make as BuildTool;

#[cfg(target_env = "msvc")]
mod msbuild;
#[cfg(target_env = "msvc")]
use msbuild::Msbuild as BuildTool;

pub type Result<T> = result::Result<T, Box<dyn Error>>;

fn main() {
    if let Err(e) = run() {
        eprintln!("\n{e}");
        exit(1);
    }
}

fn run() -> Result<()> {
    if fs::read_dir("mupdf").is_ok_and(|d| d.count() == 0) {
        Err(
            "The `mupdf` directory is empty, did you forget to pull the submodules?\n\
            Try `git submodule update --init --recursive`",
        )?
    }

    let target = Target::from_cargo().map_err(|e| {
        format!(
            "Unable to detect target: {e}\n\
            Cargo is required to build mupdf"
        )
    })?;

    let src_dir = current_dir().unwrap().join("mupdf");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    let build_dir = out_dir.join("build");
    let build_dir = build_dir.to_str().ok_or_else(|| {
        format!("Build dir path is required to be valid UTF-8, got {build_dir:?}")
    })?;

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=wrapper.c");

    Build::default().run(&target, &src_dir, build_dir)?;
    build_wrapper(&target).map_err(|e| format!("Unable to compile mupdf wrapper:\n  {e}"))?;

    generate_bindings(&out_dir.join("bindings.rs"))
        .map_err(|e| format!("Unable to generate mupdf bindings using bindgen:\n  {e}"))?;

    Ok(())
}

fn build_wrapper(target: &Target) -> Result<()> {
    let mut build = cc::Build::new();
    build.file("wrapper.c").include("mupdf/include");
    if target.os == "android" {
        build.define("HAVE_ANDROID", None);
    }
    build.try_compile("mupdf-wrapper")?;
    Ok(())
}

fn generate_bindings(path: &Path) -> Result<()> {
    let builder = bindgen::builder()
        .clang_arg("-Imupdf/include")
        .header("wrapper.h")
        .header("wrapper.c")
        .allowlist_item("fz_.*")
        .allowlist_item("FZ_.*")
        .allowlist_item("pdf_.*")
        .allowlist_item("PDF_.*")
        .allowlist_item("ucdn_.*")
        .allowlist_item("UCDN_.*")
        .allowlist_item("Memento_.*")
        .allowlist_item("mupdf_.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .parse_callbacks(Box::new(DocsCallbacks::default()));

    #[cfg(feature = "zerocopy")]
    let builder = builder.parse_callbacks(Box::new(ZerocopyDeriveCallbacks));

    builder
        .size_t_is_usize(true)
        .generate()?
        .write_to_file(path)?;

    Ok(())
}

// see https://github.com/ArtifexSoftware/mupdf/blob/master/source/fitz/noto.c
const FONTS: [&str; 6] = [
    "TOFU",
    "TOFU_CJK",
    "TOFU_NOTO",
    "TOFU_SYMBOL",
    "TOFU_EMOJI",
    "TOFU_SIL",
];

#[derive(Default)]
struct Build {
    tool: BuildTool,
}

impl Build {
    fn define_bool(&mut self, var: &str, val: bool) {
        self.tool.define(var, if val { "1" } else { "0" });
    }

    fn fz_enable(&mut self, name: &str, enable: bool) {
        self.define_bool(&format!("FZ_ENABLE_{name}"), enable);
    }

    fn run(mut self, target: &Target, src_dir: &Path, build_dir: &str) -> Result<()> {
        self.fz_enable("XPS", cfg!(feature = "xps"));
        self.fz_enable("SVG", cfg!(feature = "svg"));
        self.fz_enable("CBZ", cfg!(feature = "cbz"));
        self.fz_enable("IMG", cfg!(feature = "img"));
        self.fz_enable("HTML", cfg!(feature = "html"));
        self.fz_enable("EPUB", cfg!(feature = "epub"));
        self.fz_enable("JS", cfg!(feature = "js"));

        for font in &FONTS {
            self.define_bool(font, cfg!(feature = "all-fonts"));
        }

        self.tool.build(target, src_dir, build_dir)
    }
}

#[allow(dead_code)]
struct Target {
    debug: bool,
    opt_level: String,
    arch: String,
    os: String,
    features: Vec<String>,
}

impl Target {
    fn from_cargo() -> Result<Self> {
        Ok(Self {
            debug: env::var_os("DEBUG").is_some_and(|s| s != "0" && s != "false"),
            opt_level: env::var("OPT_LEVEL")?,
            arch: env::var("CARGO_CFG_TARGET_ARCH")?,
            os: env::var("CARGO_CFG_TARGET_OS")?,
            features: env::var("CARGO_CFG_TARGET_FEATURE")?
                .split(',')
                .map(str::to_owned)
                .collect(),
        })
    }

    fn build_profile(&self) -> BuildProfile {
        match &*self.opt_level {
            _ if self.debug => BuildProfile::Debug,
            "2" | "3" => BuildProfile::Release,
            "s" | "z" => BuildProfile::Small,
            _ => BuildProfile::Debug,
        }
    }
}

enum BuildProfile {
    Debug,
    Release,
    Small,
}

#[cfg(feature = "zerocopy")]
#[derive(Debug)]
struct ZerocopyDeriveCallbacks;

#[cfg(feature = "zerocopy")]
impl bindgen::callbacks::ParseCallbacks for ZerocopyDeriveCallbacks {
    fn add_derives(&self, info: &bindgen::callbacks::DeriveInfo<'_>) -> Vec<String> {
        const TYPES: [&str; 2] = ["fz_point", "fz_quad"];

        if TYPES.contains(&info.name) {
            vec![
                "zerocopy::FromBytes".to_owned(),
                "zerocopy::IntoBytes".to_owned(),
                "zerocopy::Immutable".to_owned(),
            ]
        } else {
            vec![]
        }
    }
}
