mod parser;
mod presenter;
mod printer;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use parser::parse;
use parser::Patient;
use presenter::{as_csv, as_json};
use printer::{print_catalog, print_error_messages};
#[cfg(feature = "rayon")]
use rayon::prelude::*;
use serde::Serialize;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

#[derive(clap::ValueEnum, Clone, Default, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Json,
    Csv,
}

/// Simple program to catalogue DICOM files
#[derive(Parser, Debug)]
#[command(name = "pardi")]
#[command(version = "0.0.1")]
#[command(about = "Catalog DICOM files for a given path", long_about = None)]
struct Args {
    /// Path of the directory to scan
    #[arg(short, long, default_value = ".")]
    path: PathBuf,

    /// Format of the catalog
    #[arg(short, long, default_value_t, value_enum)]
    format: OutputFormat,

    /// Output file
    #[structopt(short, long)]
    output: Option<PathBuf>,

    /// Print debug information to screen
    #[arg(short, long)]
    debug: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    path_exists(&args.path)?;

    #[cfg(feature = "rayon")]
    let patients: Vec<Patient> = WalkDir::new(&args.path)
        .into_iter()
        .par_bridge()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            seek_patient(&entry, args.debug)
        })
        .collect::<Result<Vec<Patient>>>()?;

    #[cfg(not(feature = "rayon"))]
    let patients: Vec<Patient> = WalkDir::new(&args.path)
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            seek_patient(&entry, args.debug)
        })
        .collect::<Result<Vec<Patient>>>()?;

    match args.format {
        OutputFormat::Json => print_catalog(as_json(&patients), &args.output)?,
        OutputFormat::Csv => print_catalog(as_csv(&patients), &args.output)?,
    }

    Ok(())
}

fn seek_patient(entry: &DirEntry, debug: bool) -> Option<Result<Patient>> {
    match parse(entry) {
        Ok(patient) => Some(Ok(patient)),
        Err(err) => {
            print_error_messages(err, debug);
            None
        }
    }
}

fn path_exists(path_buff: &Path) -> Result<()> {
    if !path_buff.exists() {
        return Err(anyhow!("Path does not exists")).context(format!(
            "the path {} can not be found on the filesystem",
            path_buff.display()
        ));
    }
    Ok(())
}
