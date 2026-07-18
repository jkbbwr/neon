//! The back half of every build verb: take a checked program, run the IR pipeline, emit C
//! next to the output, and hand it to the configured C compiler along with the runtime.

use crate::buildcfg::BuildConfig;
use crate::frontend::Checked;
use color_eyre::eyre::{bail, eyre, Result};
use neon_compiler::backend::c;
use neon_compiler::ir::{self, Stage};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Lower a checked program to an executable at `out`, writing a sibling `.c` file.
pub fn to_executable(checked: &Checked, out: &Path, cfg: &BuildConfig) -> Result<()> {
    let libs: Vec<(Vec<String>, &_)> =
        checked.libs.iter().map(|(p, m)| (p.clone(), m)).collect();
    let program = ir::compile(&checked.env, &checked.result, &checked.module, &libs, Stage::Final);
    let c_source = c::emit(&program);

    let c_file = out.with_extension("c");
    std::fs::write(&c_file, &c_source).map_err(|e| eyre!("writing {}: {e}", c_file.display()))?;

    let (include, rt_c) = runtime_sources()?;
    let mut cmd = Command::new(&cfg.cc);
    cmd.args(cfg.cc_args())
        .arg("-o")
        .arg(out)
        .arg(&c_file)
        .arg(&rt_c)
        .arg("-I")
        .arg(&include);
    let status = cmd
        .status()
        .map_err(|e| eyre!("could not run the C compiler `{}`: {e}", cfg.cc))?;
    if !status.success() {
        bail!("the C compiler failed on {}", c_file.display());
    }
    Ok(())
}

/// The runtime's include directory and `rt.c`, found under the sysroot.
fn runtime_sources() -> Result<(PathBuf, PathBuf)> {
    let root = match std::env::var_os("NEON_SYSROOT") {
        Some(dir) => PathBuf::from(dir).join("runtime"),
        None => {
            let exe = std::env::current_exe()?;
            exe.parent()
                .ok_or_else(|| eyre!("the neon binary has no parent directory"))?
                .join("../runtime")
        }
    };
    let include = root.join("include");
    let rt_c = root.join("src/rt.c");
    if !rt_c.is_file() {
        bail!("cannot find the runtime at {}", root.display());
    }
    Ok((include, rt_c))
}
