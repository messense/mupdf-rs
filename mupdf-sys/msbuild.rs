use std::{env, fs, path::Path};

use cc::windows_registry::{self, find_vs_version, VsVers};

use crate::{Result, Target};

#[derive(Default)]
pub struct Msbuild {
    cl: Vec<String>,
}

impl Msbuild {
    pub fn define(&mut self, var: &str, val: &str) {
        self.cl.push(format!("/D{var}#{val}"));
    }

    pub fn build(mut self, target: &Target, build_dir: &str) -> Result<()> {
        self.cl.push("/MP".to_owned());

        // work around https://developercommunity.visualstudio.com/t/NAN-is-no-longer-compile-time-constant-i/10688907
        let file_path = Path::new(build_dir).join("source/fitz/geometry.c");
        let content = fs::read_to_string(&file_path).expect("Failed to read geometry.c file");
        let patched_content = content.replace("NAN", "(0.0/0.0)");
        fs::write(&file_path, patched_content).expect("Failed to write patched geometry.c file");

        let configuration = if target.debug_profile() {
            "Debug"
        } else {
            "Release"
        };

        let platform = match &*target.arch {
            "i386" | "i586" | "i686" => "Win32",
            "x86_64" => "x64",
            _ => Err(format!(
                "mupdf currently only supports Win32 and x64 with msvc\n\
                Try compiling using mingw for potential {:?} support",
                target.arch,
            ))?,
        };

        let platform_toolset = env::var("MUPDF_MSVC_PLATFORM_TOOLSET").unwrap_or_else(|_| {
            match find_vs_version() {
                Ok(VsVers::Vs17) => "v143",
                _ => "v142",
            }
            .to_owned()
        });

        let Some(mut msbuild) = windows_registry::find(&target.arch, "msbuild.exe") else {
            Err("Could not find msbuild.exe. Do you have it installed?")?
        };
        let status = msbuild
            .args([
                r"platform\win32\mupdf.sln",
                "/target:libmupdf",
                &format!("/p:Configuration={configuration}"),
                &format!("/p:Platform={platform}"),
                &format!("/p:PlatformToolset={platform_toolset}"),
            ])
            .current_dir(build_dir)
            .env("CL", self.cl.join(" "))
            .status()
            .map_err(|e| format!("Failed to call msbuild: {e}"))?;
        if !status.success() {
            Err(match status.code() {
                Some(code) => format!("msbuild invocation failed with status {code}"),
                None => "msbuild invocation failed".to_owned(),
            })?;
        }

        if platform == "x64" {
            println!(
                "cargo:rustc-link-search=native={build_dir}/platform/win32/x64/{configuration}"
            );
        } else {
            println!("cargo:rustc-link-search=native={build_dir}/platform/win32/{configuration}");
        }

        if configuration == "Debug" {
            println!("cargo:rustc-link-lib=dylib=ucrtd");
            println!("cargo:rustc-link-lib=dylib=vcruntimed");
            println!("cargo:rustc-link-lib=dylib=msvcrtd");
        }

        println!("cargo:rustc-link-lib=dylib=libmupdf");
        println!("cargo:rustc-link-lib=dylib=libthirdparty");

        Ok(())
    }
}
