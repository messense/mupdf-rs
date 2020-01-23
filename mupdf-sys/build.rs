use std::env;
use std::fs;
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

    let out_dir = env::var("OUT_DIR").unwrap();
    let current_dir = env::current_dir().unwrap();
    let mupdf_dir = current_dir.join("mupdf");
    let output = Command::new("make")
        .arg(format!("OUT={}", out_dir))
        .arg("USE_SYSTEM_LIBS=no")
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
}
