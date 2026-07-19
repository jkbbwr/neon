//! The back half of every build verb: take a checked program, run the IR pipeline, emit C
//! next to the output, and hand it to the configured C compiler along with the runtime.

use crate::buildcfg::BuildConfig;
use crate::frontend::Checked;
use crate::sysroot::Sysroot;
use color_eyre::eyre::{bail, eyre, Result};
use neon_compiler::backend::c;
use neon_compiler::ir::{self, Stage};
use std::path::Path;
use std::process::Command;

/// Lower a checked program to an executable at `out`, writing a sibling `.c` file.
pub fn to_executable(checked: &Checked, out: &Path, cfg: &BuildConfig) -> Result<()> {
    let libs: Vec<(Vec<String>, &_)> =
        checked.libs.iter().map(|(p, m)| (p.clone(), m)).collect();
    let program = ir::compile(&checked.env, &checked.result, &checked.module, &libs, Stage::Final);
    let c_source = c::emit(&program);

    let c_file = out.with_extension("c");
    std::fs::write(&c_file, &c_source).map_err(|e| eyre!("writing {}: {e}", c_file.display()))?;

    // The runtime is a prebuilt archive, not a pile of `.c` files: one variant per build
    // shape, built once by cmake (`runtime/CMakeLists.txt`). A build used to recompile
    // all eleven runtime translation units every time, and a shipped toolchain would have
    // had to ship the runtime's C source. `runtime_variant` decides which archive, and
    // refuses rather than substituting one — see its doc comment for why a sanitized
    // build may not link an uninstrumented runtime.
    let sysroot = Sysroot::find()?;
    let variant = cfg.runtime_variant()?;
    let archive = sysroot.runtime_lib(variant)?;
    // Asking for a strict subset of the sanitized archive's sanitizers links the full set
    // instead — safe, but not something to do behind the user's back.
    if let Some(note) = cfg.sanitizer_widening_note(variant) {
        eprintln!("{note}");
    }
    let mut cmd = Command::new(&cfg.cc);
    cmd.args(cfg.cc_args(variant))
        .arg("-o")
        .arg(out)
        .arg(&c_file)
        // After the object that references it: a static archive only contributes the
        // members that resolve symbols already seen.
        .arg(&archive)
        .arg("-I")
        .arg(sysroot.include());
    let status = cmd
        .status()
        .map_err(|e| eyre!("could not run the C compiler `{}`: {e}", cfg.cc))?;
    if !status.success() {
        bail!("the C compiler failed on {}", c_file.display());
    }
    Ok(())
}
