use crate::buildcfg::RuntimeVariant;
use crate::sysroot::Sysroot;
use color_eyre::eyre::{bail, Context, Result};

/// Print the stdlib directory alone, one absolute path on one line, nothing else.
///
/// The machine-readable half of this command, and the language server's only way in: the
/// LSP shells out to `neon sysroot --stdlib` once at startup rather than linking a second
/// copy of the discovery logic, so it reports the toolchain the user actually builds with
/// — whatever `neon` is on PATH — instead of whatever a linked-in copy would compute.
/// That keeps one source of truth even when the LSP and CLI binaries are different builds.
///
/// It goes through `stdlib_dir` rather than `find`, deliberately: type-checking needs only
/// the stdlib source, so demanding `lib/libneon_rt.a` here would make the editor useless on
/// a toolchain whose runtime has not been built. The directory is verified to exist,
/// because `stdlib_dir` composes a path without probing and a phantom path handed to the
/// LSP would come back as "every stdlib name is unknown" rather than as an error.
fn print_stdlib() -> Result<()> {
    let dir = Sysroot::stdlib_dir().wrap_err("failed to locate the toolchain")?;
    if !dir.is_dir() {
        bail!(
            "no stdlib directory at '{}'.\n\
             Set NEON_SYSROOT to a sysroot containing `stdlib/`.",
            dir.display()
        );
    }
    println!("{}", dir.display());
    Ok(())
}

pub fn run(stdlib_only: bool) -> Result<()> {
    if stdlib_only {
        return print_stdlib();
    }
    let s = Sysroot::find().wrap_err("failed to locate the toolchain")?;
    println!("{}", s.root().display());
    println!("  include: {}", s.include().display());
    println!("  stdlib:  {}", s.stdlib().display());
    // All three runtime variants, present or not: which ones exist decides which builds
    // this toolchain can do at all (no sanitized archive means no sanitized build).
    println!("  runtime variants in {}:", s.lib_dir().display());
    for v in [RuntimeVariant::Release, RuntimeVariant::Debug, RuntimeVariant::Sanitized] {
        let mark = if s.lib_dir().join(v.archive()).is_file() { "present" } else { "MISSING" };
        println!("    {:<20} {mark}", v.archive());
    }
    Ok(())
}
