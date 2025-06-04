use std::env::{self, current_dir};
use std::error::Error;
use std::ffi::OsStr;
use std::fs::remove_dir_all;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::{fs, result};

mod docs;
use docs::DocsCallbacks;

mod make;
use make::Make;

mod msbuild;
use msbuild::Msbuild;

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
    let out_dir =
        PathBuf::from(env::var_os("OUT_DIR").ok_or("Missing OUT_DIR environment variable")?);

    let sysroot = find_clang_sysroot(&target)?;

    let docs = env::var_os("DOCS_RS").is_some();
    if !docs {
        let build_dir = out_dir.join("build");
        let build_dir = build_dir.to_str().ok_or_else(|| {
            format!("Build dir path is required to be valid UTF-8, got {build_dir:?}")
        })?;

        if let Err(e) = remove_dir_all(build_dir) {
            if e.kind() != ErrorKind::NotFound {
                println!("cargo:warning=Unable to clear {build_dir:?}. This may lead to flaky builds that might not incorporate configurations changes: {e}");
            }
        }

        copy_recursive(&src_dir, build_dir.as_ref(), &[".git".as_ref()])?;

        println!("cargo:rerun-if-changed=wrapper.h");
        println!("cargo:rerun-if-changed=wrapper.c");

        Build::new(&target).run(&target, build_dir)?;
        build_wrapper(&target).map_err(|e| format!("Unable to compile mupdf wrapper:\n  {e}"))?;
    }

    generate_bindings(&target, &out_dir.join("bindings.rs"), sysroot)
        .map_err(|e| format!("Unable to generate mupdf bindings using bindgen:\n  {e}"))?;

    Ok(())
}

fn copy_recursive(src: &Path, dst: &Path, ignore: &[&OsStr]) -> Result<()> {
    if let Err(e) = fs::create_dir(dst) {
        if e.kind() != ErrorKind::AlreadyExists {
            Err(format!("Unable to create {dst:?}: {e}"))?;
        }
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        if ignore.contains(&&*entry.file_name()) {
            continue;
        }

        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        let file_type = entry.file_type()?;

        #[cfg(any(unix, windows))]
        if file_type.is_symlink() {
            let link = fs::read_link(&src_path)
                .map_err(|e| format!("Couldn't read symlink {src_path:?}: {e}"))?;

            #[cfg(unix)]
            let err = std::os::unix::fs::symlink(&link, &dst_path);
            #[cfg(windows)]
            let err = if file_type.is_dir() {
                std::os::windows::fs::symlink_dir(&link, &dst_path)
            } else {
                std::os::windows::fs::symlink_file(&link, &dst_path)
            };

            match err {
                Ok(_) => continue,
                Err(e) => println!(
                    "cargo:warning=Couldn't create symlink {dst_path:?} pointing to {link:?}. This might increase the size of your target folder: {e}"
                ),
            }
        }

        if file_type.is_file() || fs::metadata(&src_path)?.is_file() {
            fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Couldn't copy {src_path:?} to {dst_path:?}: {e}"))?;
        } else {
            copy_recursive(&src_path, &dst_path, ignore)?;
        }
    }
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

fn find_clang_sysroot(target: &Target) -> Result<Option<String>> {
    if target.os == "emscripten" {
        let sdk = env::var("EMSDK").map_err(|e| match e {
            env::VarError::NotPresent => {
                "Using emscripten requires the EMSDK environment variable to be set".to_owned()
            }
            _ => {
                format!("Invalid EMSDK environment variable: {e}")
            }
        })?;

        let mut sysroot = PathBuf::from(sdk);
        sysroot.push("upstream/emscripten/cache/sysroot");
        let sysroot = sysroot.into_os_string().into_string().unwrap();
        return Ok(Some(sysroot));
    }

    Ok(None)
}

fn generate_bindings(target: &Target, path: &Path, sysroot: Option<String>) -> Result<()> {
    let mut builder = bindgen::builder();

    if let Some(sysroot) = sysroot {
        builder = builder.clang_arg("--sysroot").clang_arg(sysroot);
    }

    if target.os == "emscripten" {
        builder = builder.clang_arg("-fvisibility=default");
    }

    builder = builder
        .clang_arg("-Imupdf/include")
        .header("wrapper.h")
        .header("wrapper.c");

    builder = builder
        .allowlist_recursively(false)
        .allowlist_type("wchar_t")
        .allowlist_type("FILE")
        .opaque_type("FILE");

    builder = builder
        .allowlist_item("fz_.*")
        .allowlist_item("FZ_.*")
        .allowlist_item("pdf_.*")
        .allowlist_item("PDF_.*")
        .allowlist_type("cmap_splay")
        .allowlist_item("ucdn_.*")
        .allowlist_item("UCDN_.*")
        .allowlist_item("Memento_.*")
        .allowlist_item("mupdf_.*");

    // remove va_list functions as for all of these versions using ... exist
    builder = builder
        .blocklist_function("Memento_vasprintf") // Memento_asprintf
        .blocklist_function("fz_vthrow") // fz_throw
        .blocklist_function("fz_vwarn") // fz_warn
        .blocklist_function("fz_vlog_error_printf") // fz_log_error_printf
        .blocklist_function("fz_append_vprintf") // fz_append_printf
        .blocklist_function("fz_write_vprintf") // fz_write_printf
        .blocklist_function("fz_vsnprintf") // fz_snprintf
        .blocklist_function("fz_format_string"); // mupdf_format_string

    // build config
    builder = builder
        .blocklist_var("FZ_VERSION.*")
        .blocklist_var("FZ_ENABLE_.*")
        .blocklist_var("FZ_PLOTTERS_.*");

    // internal implementation details, considered private
    builder = builder
        .blocklist_item("fz_jmp_buf")
        .blocklist_function("fz_var_imp")
        .blocklist_function("fz_push_try")
        .blocklist_function("fz_do_.*")
        .blocklist_var("FZ_JMPBUF_ALIGN")
        .blocklist_type("fz_error_stack_slot")
        .blocklist_type("fz_error_context")
        .blocklist_type("fz_warn_context")
        .blocklist_type("fz_aa_context")
        .blocklist_type("fz_activity_.*")
        .blocklist_function("fz_register_activity_logger")
        .opaque_type("fz_context")
        .blocklist_type("fz_new_context_imp")
        .blocklist_type("fz_lock")
        .blocklist_type("fz_unlock");

    builder = builder
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .parse_callbacks(Box::new(DocsCallbacks::default()));

    #[cfg(feature = "zerocopy")]
    {
        builder = builder.parse_callbacks(Box::new(ZerocopyDeriveCallbacks));
    }

    builder.use_core().generate()?.write_to_file(path)?;

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

enum Build {
    Make(Make),
    Msbuild(Msbuild),
}

impl Build {
    fn new(target: &Target) -> Self {
        if target.env == "msvc" {
            Self::Msbuild(Msbuild::default())
        } else {
            Self::Make(Make::default())
        }
    }

    fn define(&mut self, var: &str, val: &str) {
        match self {
            Self::Make(m) => m.define(var, val),
            Self::Msbuild(m) => m.define(var, val),
        };
    }

    fn define_bool(&mut self, var: &str, val: bool) {
        self.define(var, if val { "1" } else { "0" });
    }

    fn fz_enable(&mut self, name: &str, enable: bool) {
        self.define_bool(&format!("FZ_ENABLE_{name}"), enable);
    }

    fn run(mut self, target: &Target, build_dir: &str) -> Result<()> {
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

        match self {
            Self::Make(m) => m.build(target, build_dir),
            Self::Msbuild(m) => m.build(target, build_dir),
        }
    }
}

struct Target {
    debug: bool,
    opt_level: String,

    arch: String,
    os: String,
    env: String,

    features: Vec<String>,
}

impl Target {
    fn from_cargo() -> Result<Self> {
        Ok(Self {
            debug: env::var_os("DEBUG").is_some_and(|s| s != "0" && s != "false"),
            opt_level: env::var("OPT_LEVEL")?,

            arch: env::var("CARGO_CFG_TARGET_ARCH")?,
            os: env::var("CARGO_CFG_TARGET_OS")?,
            env: env::var("CARGO_CFG_TARGET_ENV")?,

            features: env::var("CARGO_CFG_TARGET_FEATURE")?
                .split(',')
                .map(str::to_owned)
                .collect(),
        })
    }

    fn small_profile(&self) -> bool {
        !self.debug && matches!(&*self.opt_level, "s" | "z")
    }

    fn debug_profile(&self) -> bool {
        self.debug && !matches!(&*self.opt_level, "2" | "3")
    }
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
