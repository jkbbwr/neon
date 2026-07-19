use crate::buildcfg::RuntimeVariant;
use color_eyre::eyre::{bail, eyre, Result};
use std::path::PathBuf;

/// Locates `include/`, the `lib/libneon_rt*.a` variants and `stdlib/`.
///
/// Resolved at runtime, never baked in: a compile-time path describes the
/// machine that built the compiler, not the one running it.
pub struct Sysroot(PathBuf);

impl Sysroot {
    fn probe(dir: PathBuf) -> Option<Self> {
        dir.join("lib/libneon_rt.a").is_file().then_some(Sysroot(dir))
    }

    pub fn find() -> Result<Self> {
        if let Some(dir) = std::env::var_os("NEON_SYSROOT") {
            let dir = PathBuf::from(dir);
            return Self::probe(dir.clone()).ok_or_else(|| {
                eyre!(
                    "NEON_SYSROOT is set to '{}' but there is no lib/libneon_rt.a there",
                    dir.display()
                )
            });
        }

        let exe = std::env::current_exe().map_err(|e| eyre!("cannot locate the neon binary: {e}"))?;
        let exe_dir = exe
            .parent()
            .ok_or_else(|| eyre!("the neon binary has no parent directory"))?;

        // exe_dir: dev (target/<profile>). exe_dir/..: installed (prefix/bin).
        let candidates = [exe_dir.to_path_buf(), exe_dir.join("..")];
        for dir in &candidates {
            if let Some(found) = Self::probe(dir.clone()) {
                return Ok(found);
            }
        }

        bail!(
            "cannot find the Neon sysroot: no lib/libneon_rt.a under {}.\n\
             Set NEON_SYSROOT to override.",
            candidates
                .iter()
                .map(|p| format!("'{}'", p.display()))
                .collect::<Vec<_>>()
                .join(" or ")
        )
    }

    /// The stdlib directory alone, for front-end runs that need no runtime.
    ///
    /// Probed independently of `lib/libneon_rt.a`: type-checking needs only the
    /// stdlib source, and the runtime archive does not exist until the backend does,
    /// so requiring it here would make `neon check` unusable before codegen lands.
    pub fn stdlib_dir() -> Result<PathBuf> {
        if let Some(dir) = std::env::var_os("NEON_SYSROOT") {
            return Ok(PathBuf::from(dir).join("stdlib"));
        }
        // Installed as prefix/bin/neon, so the stdlib is one directory up. A dev tree
        // has no such layout and sets NEON_SYSROOT.
        let exe = std::env::current_exe().map_err(|e| eyre!("cannot locate the neon binary: {e}"))?;
        let exe_dir = exe.parent().ok_or_else(|| eyre!("the neon binary has no parent directory"))?;
        Ok(exe_dir.join("../stdlib"))
    }

    pub fn root(&self) -> &PathBuf {
        &self.0
    }

    pub fn include(&self) -> PathBuf {
        self.0.join("include")
    }

    /// Where the prebuilt runtime archives live. The release one, `libneon_rt.a`, doubles
    /// as the marker `probe` looks for: it is the variant that always exists.
    pub fn lib_dir(&self) -> PathBuf {
        self.0.join("lib")
    }

    /// The prebuilt archive for `variant`, or an error naming what is missing.
    ///
    /// A missing archive is never quietly swapped for another. That matters most for the
    /// sanitized variant — a sanitizer reports nothing about code compiled without it, so
    /// substituting the plain archive would produce a build that looks sanitized and
    /// checks only half the program — but the same rule applies to all three, because a
    /// silently downgraded runtime is a lie about what was built either way.
    pub fn runtime_lib(&self, variant: RuntimeVariant) -> Result<PathBuf> {
        let path = self.lib_dir().join(variant.archive());
        if path.is_file() {
            return Ok(path);
        }
        let present: Vec<String> = [
            RuntimeVariant::Release,
            RuntimeVariant::Debug,
            RuntimeVariant::Sanitized,
        ]
        .iter()
        .map(|v| v.archive())
        .filter(|a| self.lib_dir().join(a).is_file())
        .map(str::to_string)
        .collect();
        bail!(
            "this build needs the runtime archive `{}`, which is not in the sysroot at {}.\n\
             Present there: {}.\n\
             The toolchain's runtime is incomplete; rebuild or reinstall it. Another \
             variant will not be substituted — it would change what the build actually \
             links without saying so.",
            variant.archive(),
            self.0.display(),
            if present.is_empty() { "nothing".into() } else { present.join(", ") },
        )
    }

    pub fn stdlib(&self) -> PathBuf {
        self.0.join("stdlib")
    }
}
