use color_eyre::eyre::{bail, eyre, Result};
use std::path::PathBuf;

/// Locates `include/`, `lib/libneon_rt.a` and `stdlib/`.
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
            let d = PathBuf::from(dir).join("stdlib");
            if d.is_dir() {
                return Ok(d);
            }
        }
        let exe = std::env::current_exe().map_err(|e| eyre!("cannot locate the neon binary: {e}"))?;
        let exe_dir = exe.parent().ok_or_else(|| eyre!("the neon binary has no parent directory"))?;
        // installed: prefix/bin/neon → prefix/stdlib. dev: target/<profile>/neon,
        // so ../../stdlib is the repo root.
        // installed prefix/bin; dev target/<profile>; and test binaries in
        // target/<profile>/deps, one level deeper again.
        let candidates = [
            exe_dir.join("stdlib"),
            exe_dir.join("../stdlib"),
            exe_dir.join("../../stdlib"),
            exe_dir.join("../../../stdlib"),
        ];
        for c in &candidates {
            if c.is_dir() {
                return Ok(c.clone());
            }
        }
        bail!(
            "cannot find the Neon stdlib under {}. Set NEON_SYSROOT to override.",
            candidates.iter().map(|p| format!("'{}'", p.display())).collect::<Vec<_>>().join(" or ")
        )
    }

    pub fn root(&self) -> &PathBuf {
        &self.0
    }

    pub fn include(&self) -> PathBuf {
        self.0.join("include")
    }

    pub fn runtime_lib(&self) -> PathBuf {
        self.0.join("lib/libneon_rt.a")
    }

    pub fn stdlib(&self) -> PathBuf {
        self.0.join("stdlib")
    }
}
