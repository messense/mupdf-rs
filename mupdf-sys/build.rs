use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

// see https://github.com/ArtifexSoftware/mupdf/blob/master/source/fitz/noto.c
const SKIP_FONTS: [&str; 6] = [
    "TOFU",
    "TOFU_CJK",
    "TOFU_NOTO",
    "TOFU_SYMBOL",
    "TOFU_EMOJI",
    "TOFU_SIL",
];

fn fail_on_empty_directory(name: &str) {
    if fs::read_dir(name).unwrap().count() == 0 {
        println!(
            "The `{}` directory is empty, did you forget to pull the submodules?",
            name
        );
        println!("Try `git submodule update --init --recursive`");
        panic!();
    }
}

macro_rules! t {
    ($e:expr) => {
        match $e {
            Ok(n) => n,
            Err(e) => panic!("\n{} failed with {}\n", stringify!($e), e),
        }
    };
}

fn cp_r(dir: &Path, dest: &Path) {
    for entry in t!(fs::read_dir(dir)) {
        let entry = t!(entry);
        let path = entry.path();
        let dst = dest.join(path.file_name().expect("Failed to get filename of path"));
        if t!(fs::metadata(&path)).is_file() {
            t!(fs::copy(path, dst));
        } else {
            t!(fs::create_dir_all(&dst));
            cp_r(&path, &dst);
        }
    }
}

#[cfg(not(target_env = "msvc"))]
fn build_libmupdf() {
    use std::process::Command;

    let profile = match &*env::var("PROFILE").unwrap_or("debug".to_owned()) {
        "bench" | "release" => "release",
        _ => "debug",
    };
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let build_dir = out_dir.join("build");
    t!(fs::create_dir_all(&build_dir));

    let current_dir = env::current_dir().unwrap();
    let mupdf_src_dir = current_dir.join("mupdf");
    cp_r(&mupdf_src_dir, &build_dir);

    // see https://github.com/ArtifexSoftware/mupdf/blob/master/include/mupdf/fitz/config.h
    let xcflags = vec![
        #[cfg(not(feature = "xps"))]
        "FZ_ENABLE_XPS=0",
        #[cfg(not(feature = "svg"))]
        "FZ_ENABLE_SVG=0",
        #[cfg(not(feature = "cbz"))]
        "FZ_ENABLE_CBZ=0",
        #[cfg(not(feature = "img"))]
        "FZ_ENABLE_IMG=0",
        #[cfg(not(feature = "html"))]
        "FZ_ENABLE_HTML=0",
        #[cfg(not(feature = "epub"))]
        "FZ_ENABLE_EPUB=0",
        #[cfg(not(feature = "js"))]
        "FZ_ENABLE_JS=0",
    ]
    .into_iter()
    .chain(SKIP_FONTS.iter().cloned())
    .map(|s| format!("-D{}", s))
    .collect::<Vec<String>>()
    .join(" ");
    let make_flags = vec![
        "libs".to_owned(),
        format!("build={}", profile),
        format!("OUT={}", build_dir.display()),
        #[cfg(feature = "sys-lib")]
        "USE_SYSTEM_LIBS=yes".to_owned(),
        "HAVE_X11=no".to_owned(),
        "HAVE_GLUT=no".to_owned(),
        "HAVE_CURL=no".to_owned(),
        "verbose=yes".to_owned(),
        format!("XCFLAGS={}", xcflags),
    ];

    #[cfg(feature = "sys-lib")]
    for lib in &[
        "freetype2",
        "zlib",
        "jbig2dec",
        "libjpeg",
        "libopenjp2",
        "gumbo",
    ] {
        let _ = pkg_config::probe_library(lib).unwrap_or_else(|e| {
            eprintln!("{}", e);
            panic!();
        });
    }

    let output = Command::new("make")
        .args(&make_flags)
        .current_dir(&build_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .expect("make failed");
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let out = String::from_utf8_lossy(&output.stdout);
        panic!("Build error:\nSTDERR:{}\nSTDOUT:{}", err, out);
    }
    println!("cargo:rustc-link-search=native={}", build_dir.display());
    println!("cargo:rustc-link-lib=static=mupdf");
    // println!("cargo:rustc-link-lib=static=mupdf-pkcs7");
    println!("cargo:rustc-link-lib=static=mupdf-third");
    // println!("cargo:rustc-link-lib=static=mupdf-threads");
}

#[cfg(target_env = "msvc")]
fn build_libmupdf() {
    let target = env::var("TARGET").expect("TARGET not found in environment");
    let msvc_platform = if target.contains("x86_64") {
        "x64"
    } else {
        "Win32"
    };
    let profile = match &*env::var("PROFILE").unwrap_or("debug".to_owned()) {
        "bench" | "release" => "Release",
        _ => "Debug",
    };
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let build_dir = out_dir.join("build");
    t!(fs::create_dir_all(&build_dir));

    let current_dir = env::current_dir().unwrap();
    let mupdf_src_dir = current_dir.join("mupdf");
    cp_r(&mupdf_src_dir, &build_dir);

    let devenv = cc::windows_registry::find(target.as_str(), "devenv.exe");
    if let Some(mut devenv) = devenv {
        let d = devenv
            .args(&["/upgrade", "platform\\win32\\mupdf.sln"])
            .current_dir(&build_dir)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .expect("failed to run devenv. Do you have it installed?");
        if !d.status.success() {
            let err = String::from_utf8_lossy(&d.stderr);
            let out = String::from_utf8_lossy(&d.stdout);
            eprintln!("Upgrade error:\nSTDERR:{}\nSTDOUT:{}", err, out);
        }
        let d = cc::windows_registry::find(target.as_str(), "devenv.exe")
            .unwrap()
            .args(&[
                "platform\\win32\\mupdf.sln",
                "/build",
                &format!("{}|{}", profile, msvc_platform),
                "/project",
                "libmupdf",
            ])
            .current_dir(&build_dir)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .expect("failed to run devenv. Do you have it installed?");
        if !d.status.success() {
            let err = String::from_utf8_lossy(&d.stderr);
            let out = String::from_utf8_lossy(&d.stdout);
            panic!("Build error:\nSTDERR:{}\nSTDOUT:{}", err, out);
        }
        if msvc_platform == "x64" {
            println!(
                "cargo:rustc-link-search=native={}/platform/win32/{}/{}",
                build_dir.display(),
                msvc_platform,
                profile
            );
        } else {
            println!(
                "cargo:rustc-link-search=native={}/platform/win32/{}",
                build_dir.display(),
                profile
            );
        }
        println!("cargo:rustc-link-lib=dylib=libmupdf");
        println!("cargo:rustc-link-lib=dylib=libthirdparty");
    } else {
        eprintln!("failed to find devenv. Do you have it installed?");
    }
}

fn main() {
    fail_on_empty_directory("mupdf");
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=wrapper.c");

    build_libmupdf();

    let mut build = cc::Build::new();
    build.file("wrapper.c");
    build.include("./mupdf/include");
    build.compile("libmupdf-wrapper.a");
    println!("cargo:rustc-link-lib=static=mupdf-wrapper");

    let bindings = bindgen::Builder::default()
        .clang_arg("-I./mupdf/include")
        .header("wrapper.h")
        .header("wrapper.c")
        .whitelist_function("fz_.*")
        .whitelist_function("pdf_.*")
        .whitelist_function("ucdn_.*")
        .whitelist_function("Memento_.*")
        .whitelist_function("mupdf_.*")
        .whitelist_type("fz_.*")
        .whitelist_type("pdf_.*")
        .whitelist_var("fz_.*")
        .whitelist_var("FZ_.*")
        .whitelist_var("pdf_.*")
        .whitelist_var("PDF_.*")
        .whitelist_var("UCDN_.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .size_t_is_usize(true)
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
