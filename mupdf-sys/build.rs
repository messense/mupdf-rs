use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

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

fn main() {
    fail_on_empty_directory("mupdf");
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=wrapper.c");

    let profile = match &*env::var("PROFILE").unwrap_or("debug".to_owned()) {
        "bench" | "release" => "release",
        _ => "debug",
    };
    let out_dir = env::var("OUT_DIR").unwrap();
    let current_dir = env::current_dir().unwrap();
    let mupdf_dir = current_dir.join("mupdf");
    // see https://github.com/ArtifexSoftware/mupdf/blob/master/include/mupdf/fitz/config.h
    // and https://github.com/ArtifexSoftware/mupdf/blob/master/source/fitz/noto.c
    let xcflags = vec![
        #[cfg(feature = "noto-small")]
        "NOTO_SMALL",
        #[cfg(feature = "no-cjk")]
        "NO_CJK",
        #[cfg(feature = "tofu")]
        "TOFU",
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
    .map(|s| {
        let mut s = s.to_owned();
        s.insert_str(0, "-D");
        s
    })
    .collect::<Vec<String>>()
    .join(" ");
    let make_flags = vec![
        "libs".to_owned(),
        format!("build={}", profile),
        format!("OUT={}", out_dir),
        #[cfg(feature = "sys-lib")]
        "USE_SYSTEM_LIBS=yes".to_owned(),
        #[cfg(feature = "x11")]
        "HAVE_X11=yes".to_owned(),
        #[cfg(feature = "opengl")]
        "HAVE_GLUT=yes".to_owned(),
        #[cfg(feature = "curl")]
        "HAVE_CURL=yes".to_owned(),
        "verbose=yes".to_owned(),
        format!("XCFLAGS={}", xcflags),
    ];
    let output = Command::new("make")
        .args(&make_flags)
        .current_dir(mupdf_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .expect("make failed");
    if output.status.code() != Some(0) {
        panic!("{:?}", String::from_utf8(output.stdout).unwrap());
    }
    #[cfg(feature = "sys-lib")]
    {
        println!("cargo:rustc-link-lib=freetype");
        println!("cargo:rustc-link-lib=z");
        println!("cargo:rustc-link-lib=jbig2dec");
        println!("cargo:rustc-link-lib=jpeg");
        println!("cargo:rustc-link-lib=openjp2");
    }
    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static=mupdf");
    // println!("cargo:rustc-link-lib=static=mupdf-pkcs7");
    println!("cargo:rustc-link-lib=static=mupdf-third");
    // println!("cargo:rustc-link-lib=static=mupdf-threads");

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
