//! lava-forge CLI — generate tatara-lisp source from a Terraform
//! provider schema JSON dump.
//!
//! Usage:
//!   lava-forge generate --schema <schema.json> --out <dir>
//!
//! Get the input via:
//!   terraform providers schema -json > schema.json

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "lava-forge",
    about = "Tatara-lisp source generator for lava providers",
    long_about = "Reads a Terraform provider schema JSON file (the output of \
                  `terraform providers schema -json`) and emits typed \
                  (deflava-resource ...) forms — one .tlisp file per resource, \
                  one _index.tlisp aggregator per provider."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Generate tatara-lisp source from a provider schema JSON file.
    Generate {
        /// Path to the provider schema JSON file
        /// (output of `terraform providers schema -json`).
        #[arg(long)]
        schema: PathBuf,
        /// Output directory. Will be created if missing; per-provider
        /// subdirectories appear inside (`<out>/aws/`, `<out>/gcp/`).
        #[arg(long)]
        out: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Generate { schema, out } => {
            let written = lava_forge::run_from_file(&schema, &out)?;
            for p in &written {
                println!("{}", p.display());
            }
            eprintln!("\nwrote {} files to {}", written.len(), out.display());
        }
    }
    Ok(())
}
