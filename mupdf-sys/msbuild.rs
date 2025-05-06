use std::{env, path::Path};

use cc::windows_registry::{self, find_vs_version};

use crate::{BuildProfile, Result, Target};

#[derive(Default)]
pub struct Msbuild {
    cl: Vec<String>,
}

impl Msbuild {
    pub fn define(&mut self, var: &str, val: &str) {
        self.cl.push(format!("/D{var}#{val}"));
    }

    pub fn build(self, target: &Target, src_dir: &Path, build_dir: &str) -> Result<()> {
        let profile = target.build_profile();
        let configuration = match profile {
            BuildProfile::Debug => "Debug",
            BuildProfile::Release => "Release",
            BuildProfile::Small => "Small",
        };

        let platform_toolset = env::var("MUPDF_MSVC_PLATFORM_TOOLSET").unwrap_or_else(|_| {
            if find_vs_version() == Ok(cc::windows_registry::VsVers::Vs17) {
                "v143"
            } else {
                "v142"
            }
            .to_owned()
        });

        let platform = match &*target.arch {
            "i686" => "Win32",
            "x86_64" => "x64",
            _ => Err("mupdf currently only supports Win32 and x64 with msvc\n\
                Try compiling using mingw to compile for other architectures")?,
        };

        let Some(mut msbuild) = windows_registry::find(&target.arch, "msbuild.exe") else {
            Err("Could not find msbuild.exe. Do you have it installed?")?
        };
        let status = msbuild
            .args([
                r"platform\win32\mupdf.sln",
                "/target:libmupdf",
                &format!("/p:OutputPath={build_dir}"),
                &format!("/p:Configuration={configuration}"),
                &format!("/p:Platform={platform}"),
                &format!("/p:PlatformToolset={platform_toolset}"),
                "/MP",
            ])
            .current_dir(src_dir)
            .env("CL", self.cl.join(" "))
            .status()
            .map_err(|e| format!("Failed to call msbuild: {e}"))?;
        if !status.success() {
            Err(match status.code() {
                Some(code) => format!("msbuild invocation failed with status {code}"),
                None => "msbuild invocation failed".to_owned(),
            })?;
        }
        match &*target.arch {
            "i686" => println!("cargo:rustc-link-search=native={build_dir}/platform/win32/{platform}/{configuration}"),
            "x86_64" => println!(
                "cargo:rustc-link-search=native={build_dir}/platform/win32/{configuration}"
            ),
            _ => {}
        };

        if matches!(profile, BuildProfile::Debug) {
            println!("cargo:rustc-link-lib=dylib=ucrtd");
            println!("cargo:rustc-link-lib=dylib=vcruntimed");
            println!("cargo:rustc-link-lib=dylib=msvcrtd");
        }

        println!("cargo:rustc-link-lib=dylib=libmupdf");
        println!("cargo:rustc-link-lib=dylib=libthirdparty");

        Ok(())
    }
}
