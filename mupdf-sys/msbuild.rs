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

    fn patch_nan(&self, build_dir: &str) -> Result<()> {
        let file_path = Path::new(build_dir).join("source/fitz/geometry.c");
        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read geometry.c: {e}"))?;

        // work around https://developercommunity.visualstudio.com/t/NAN-is-no-longer-compile-time-constant-i/10688907
        let patched_content = content.replace("NAN", "(0.0/0.0)");

        fs::write(&file_path, patched_content)
            .map_err(|e| format!("Failed to write patched geometry.c: {e}"))?;

        Ok(())
    }

    fn remove_libresources_fonts(&self, build_dir: &str) -> Result<()> {
        let file_path = Path::new(build_dir).join("platform/win32/libresources.vcxproj");
        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read libresources.vcxproj: {e}"))?;

        let patched: String = content
            .lines()
            .filter(|line| {
                !line.contains(r"fonts\han\")
                    && !line.contains(r"fonts\droid\")
                    && !line.contains(r"fonts\noto\")
                    && !line.contains(r"fonts\sil\")
            })
            .collect::<Vec<_>>()
            .join("\n");

        fs::write(&file_path, patched)
            .map_err(|e| format!("Failed to write patched libresources.vcxproj: {e}"))?;

        Ok(())
    }

    /// Honor the Cargo feature set on the MSBuild path.
    ///
    /// MuPDF's generated `libmupdf.vcxproj` (the `Release`/`Debug` config we build)
    /// hard-codes OCR ON (`HAVE_TESSERACT;HAVE_LEPTONICA`) and unconditionally
    /// `ProjectReference`s the barcode (`libmubarcode` → libzxing/libzint) and
    /// `extract`/docx (`libextract`) projects. mupdf-sys's matching features only
    /// reach the GNU Make path (`make_bool`), never MSBuild — so on Windows those
    /// optional libraries are built regardless of features. That wastes build time
    /// for everyone. Strip each *disabled* feature's macros and project references
    /// so MSBuild builds only what the features select; an *enabled* feature is left
    /// in place (the vcxproj already references it, and the crate vendors the
    /// source).
    ///
    /// Notes:
    /// - OCR is gated on `defined()` in config.h
    ///   (`#if !defined(HAVE_LEPTONICA) || !defined(HAVE_TESSERACT)` → `OCR_DISABLED`),
    ///   so its macros must be *removed*, not set to 0; that also stubs out
    ///   `ocr-device.c` so no Tesseract symbols are referenced. The barcode/docx
    ///   compile units are gated by `FZ_ENABLE_BARCODE`/`FZ_ENABLE_DOCX_OUTPUT`
    ///   (set from build.rs via `fz_enable`), so only their project references
    ///   need removing here.
    /// - OpenSSL/`HAVE_LIBCRYPTO` and libarchive live only in MuPDF's separate
    ///   `*Extra` configurations, not the `Release`/`Debug` ones we build, so they
    ///   need no handling (`pkcs7-openssl.c` compiles its built-in "No OpenSSL
    ///   support" stub when `HAVE_LIBCRYPTO` is undefined).
    /// - Enabling `tesseract` *with* the ClangCL toolset is a known gap: clang can't
    ///   compile MuPDF's `libtesseract.vcxproj` (its STL-heavy TUs pull clang's
    ///   `<mmintrin.h>` via `<intrin.h>` and fail). OCR-on therefore needs a real
    ///   MSVC toolset for now.
    fn exclude_disabled_features(&self, build_dir: &str) -> Result<()> {
        let mut drop_defines: Vec<&str> = Vec::new();
        let mut drop_refs: Vec<&str> = Vec::new();

        if !cfg!(feature = "tesseract") {
            drop_defines.extend(["HAVE_TESSERACT", "HAVE_LEPTONICA"]);
            drop_refs.push("libtesseract.vcxproj");
        }
        if !cfg!(feature = "zxingcpp") {
            // libmubarcode pulls libzxing (and libzint) transitively.
            drop_refs.push("libmubarcode.vcxproj");
        }
        if !cfg!(feature = "docx-output") {
            drop_refs.push("libextract.vcxproj");
        }

        // Everything selected → leave the project untouched.
        if drop_defines.is_empty() && drop_refs.is_empty() {
            return Ok(());
        }

        let vcxproj = Path::new(build_dir).join("platform/win32/libmupdf.vcxproj");
        Self::prune_vcxproj(&vcxproj, &drop_defines, &drop_refs)
    }

    /// Remove the named `;`-delimited macros from every `<PreprocessorDefinitions>`
    /// element and delete whole `<ProjectReference>` blocks whose `Include` matches
    /// `drop_refs`. Macros are matched by name (ignoring any `=value`), not by
    /// substring, so this is robust to ordering and to `FOO=1`-style values.
    fn prune_vcxproj(path: &Path, drop_defines: &[&str], drop_refs: &[&str]) -> Result<()> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;

        let mut out: Vec<String> = Vec::with_capacity(content.lines().count());
        let mut skipping = false;
        for line in content.lines() {
            if skipping {
                if line.contains("</ProjectReference>") {
                    skipping = false;
                }
                continue;
            }
            if line.contains("<ProjectReference") && drop_refs.iter().any(|r| line.contains(r)) {
                // Multi-line block unless it self-closes on this line.
                if !line.contains("</ProjectReference>") && !line.contains("/>") {
                    skipping = true;
                }
                continue;
            }
            if !drop_defines.is_empty() && line.contains("<PreprocessorDefinitions>") {
                out.push(Self::strip_defines(line, drop_defines));
            } else {
                out.push(line.to_owned());
            }
        }

        fs::write(path, out.join("\n"))
            .map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
        Ok(())
    }

    /// Rebuild a single `<PreprocessorDefinitions>` line without the dropped macros.
    fn strip_defines(line: &str, drop: &[&str]) -> String {
        const OPEN: &str = "<PreprocessorDefinitions>";
        const CLOSE: &str = "</PreprocessorDefinitions>";
        let (Some(open), Some(close)) = (line.find(OPEN), line.find(CLOSE)) else {
            return line.to_owned();
        };
        let inner = open + OPEN.len();
        if close < inner {
            return line.to_owned();
        }
        let kept: Vec<&str> = line[inner..close]
            .split(';')
            .filter(|tok| !drop.contains(&tok.split('=').next().unwrap_or(tok)))
            .collect();
        format!("{}{}{}", &line[..inner], kept.join(";"), &line[close..])
    }

    pub fn build(mut self, target: &Target, build_dir: &str) -> Result<()> {
        self.patch_nan(build_dir)?;
        self.remove_libresources_fonts(build_dir)?;

        let platform_toolset = env::var("MUPDF_MSVC_PLATFORM_TOOLSET").unwrap_or_else(|_| {
            match find_vs_version() {
                Ok(VsVers::Vs17) => "v143",
                _ => "v142",
            }
            .to_owned()
        });
        let clang_cl = platform_toolset.eq_ignore_ascii_case("ClangCL");

        // libmupdf's own deskew/skew use SSE4.1 intrinsics (smmintrin.h)
        // unconditionally on x86 (ARCH_HAS_SSE, system.h). MSVC exposes every
        // intrinsic regardless of /arch, but clang-cl gates them behind target
        // features, so those TUs fail to compile under ClangCL unless SSE4.1 is
        // enabled explicitly. SSE4.1 (⊇ SSSE3/SSE3/SSE2) is the level libmupdf
        // needs and is also MuPDF's assumed x86 baseline, so applying it build-wide
        // raises no runtime requirement. Gate on the toolset (not $CC, which MSBuild
        // doesn't set) so plain MSVC (v142/v143) builds are untouched.
        if clang_cl {
            self.cl.push("/clang:-msse4.1".to_owned());
        }

        // Cross-language LTO: emit libmupdf's TUs as LLVM ThinLTO bitcode so the
        // consuming rustc link (built with `-Clinker-plugin-lto`, clang-cl + lld)
        // can inline and optimize across the Rust<->C boundary. MSVC `/GL` can't do
        // this -- its MSIL is not LLVM bitcode and never crosses into Rust -- so
        // this is ClangCL-only; error out under any MSVC toolset. `/clang:` forwards
        // to the clang driver (same mechanism as the SSE4.1 flag above). `/GL`/WPO
        // must stay off (it is, by default) -- the two whole-program models are
        // mutually exclusive.
        if cfg!(feature = "linker-plugin-lto") {
            if !clang_cl {
                Err(
                    "the `linker-plugin-lto` feature requires the ClangCL toolset \
                     (MSVC `/GL` is not LLVM bitcode); set \
                     MUPDF_MSVC_PLATFORM_TOOLSET=ClangCL",
                )?;
            }
            self.cl.push("/clang:-flto=thin".to_owned());
        }

        // The MSBuild project graph hard-codes every optional dependency ON; strip
        // the ones whose Cargo feature is disabled (see `exclude_disabled_features`).
        self.exclude_disabled_features(build_dir)?;

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

        let Some(mut msbuild) = windows_registry::find(&target.arch, "msbuild.exe") else {
            Err("Could not find msbuild.exe. Do you have it installed?")?
        };
        let mut args = vec![
            r"platform\win32\mupdf.sln".to_owned(),
            "/target:libmupdf".to_owned(),
            format!("/p:Configuration={configuration}"),
            format!("/p:Platform={platform}"),
            format!("/p:PlatformToolset={platform_toolset}"),
        ];
        // MuPDF's Release config sets `<WholeProgramOptimization>true</…>` (`/GL`),
        // which is the wrong default for libmupdf *as built here*: it's a static lib
        // produced solely to be linked by an external (rustc) link. `/GL` defers code
        // generation to that final link, so the .lib ships version-locked MSIL objects
        // rather than finished code. Consequences for the consumer:
        //   - the final rustc link (which doesn't pass `/LTCG`) makes link.exe detect
        //     the `/GL` modules and *restart* with `/LTCG`, re-running whole-program
        //     codegen of all of libmupdf on every relink (slow; on some toolsets it
        //     ICEs, e.g. freetype t1load.c C1001 / link.exe 0xc0000005);
        //   - `lld-link` (an increasingly common Rust linker) can't consume `/GL`
        //     objects at all → hard link failure;
        //   - MS itself advises against shipping `.lib` files made of `/GL` objects.
        // The `/GL`→Rust link can't optimize across the Rust↔C boundary anyway, so the
        // only thing lost is cross-TU optimization *within* MuPDF — a real but bounded
        // runtime win. Default it off; let perf-sensitive users opt back in. A global
        // `/p:` overrides the per-config project setting. (ClangCL builds don't emit
        // MSIL for `/GL`, so they're unaffected either way.)
        if env::var_os("MUPDF_MSVC_WHOLE_PROGRAM_OPT").is_none() {
            args.push("/p:WholeProgramOptimization=false".to_owned());
        }
        let status = msbuild
            .args(&args)
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
        // NB: no separate libtesseract/libleptonica directives are needed even with
        // the `tesseract` feature — MSVC "Link Library Dependencies" merges the
        // ProjectReference'd static libs into libmupdf.lib (it carries the
        // libtesseract/libleptonica objects directly), so libmupdf alone resolves
        // all OCR symbols. Verified 2026-06-27 via dumpbin /ARCHIVEMEMBERS.

        Ok(())
    }
}
