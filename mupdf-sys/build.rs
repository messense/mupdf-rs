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

    let out_dir = env::var("OUT_DIR").unwrap();
    let current_dir = env::current_dir().unwrap();
    let mupdf_dir = current_dir.join("mupdf");
    let output = Command::new("make")
        .arg(format!("OUT={}", out_dir))
        .arg("USE_SYSTEM_LIBS=no")
        .arg("HAVE_X11=no")
        .arg("HAVE_GLUT=no")
        .current_dir(mupdf_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .expect("make failed");
    if output.status.code() != Some(0) {
        panic!("{:?}", String::from_utf8(output.stdout).unwrap());
    }
    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static=mupdf");
    println!("cargo:rustc-link-lib=static=mupdf-pkcs7");
    println!("cargo:rustc-link-lib=static=mupdf-third");
    println!("cargo:rustc-link-lib=static=mupdf-threads");

    let bindings = bindgen::Builder::default()
        .clang_arg("-I./mupdf/include")
        .header("wrapper.h")
        .whitelist_function("fz_.*")
        .whitelist_function("pdf_.*")
        .whitelist_function("ucdn_.*")
        .whitelist_type("fz_.*")
        .whitelist_type("pdf_.*")
        .whitelist_var("fz_.*")
        .whitelist_var("FZ_.*")
        .whitelist_var("pdf_.*")
        .whitelist_var("PDF_.*")
        .whitelist_var("UCDN_.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
