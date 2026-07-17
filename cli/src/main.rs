mod sysroot;

use clap::{Parser, Subcommand};
use color_eyre::eyre::{Context, Result};
use neon_compiler::lexer;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use sysroot::Sysroot;

#[derive(Parser)]
#[command(name = "neon", version, about = "The Neon toolchain")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Lex a source file and print its tokens.
    Lex {
        /// OsString, not PathBuf-from-String: a path need not be UTF-8, and
        /// rejecting one at the arg parser is a worse error than failing to
        /// open it.
        file: OsString,
        /// Print byte spans alongside each token.
        #[arg(long)]
        spans: bool,
    },
    /// Print the resolved sysroot.
    Sysroot,
}

fn read_source(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .wrap_err_with(|| format!("cannot read '{}'", path.display()))
}

fn cmd_lex(file: &OsString, spans: bool) -> Result<()> {
    let path = PathBuf::from(file);
    let src = read_source(&path)?;

    match lexer::lex(&src) {
        Ok(tokens) => {
            for t in tokens {
                if spans {
                    println!("{:>5}..{:<5} {:?}", t.span.start, t.span.end, t.token);
                } else {
                    println!("{:?}", t.token);
                }
            }
            Ok(())
        }
        Err(errors) => {
            // Every error, not just the first: the lexer accumulates so a
            // diagnostics pass can show them all.
            for e in &errors {
                eprintln!("{}:{}: error: {}", path.display(), line_of(&src, e.span.start), e);
            }
            std::process::exit(1);
        }
    }
}

/// 1-based line number for a byte offset. Placeholder until diagnostics land.
fn line_of(src: &str, offset: usize) -> usize {
    src[..offset.min(src.len())].bytes().filter(|b| *b == b'\n').count() + 1
}

fn cmd_sysroot() -> Result<()> {
    let s = Sysroot::find().wrap_err("failed to locate the toolchain")?;
    println!("{}", s.root().display());
    println!("  include: {}", s.include().display());
    println!("  runtime: {}", s.runtime_lib().display());
    println!("  stdlib:  {}", s.stdlib().display());
    Ok(())
}

fn main() -> Result<()> {
    color_eyre::install()?;
    match Cli::parse().command {
        Command::Lex { file, spans } => cmd_lex(&file, spans),
        Command::Sysroot => cmd_sysroot(),
    }
}
