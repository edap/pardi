use anyhow::{anyhow, Context, Result};
use clap::Parser;
use pardi::parser::{parse, Record, DEFAULT_FIELDS};
use pardi::presenter::StreamWriter;
use pardi::printer::print_error_messages;
use pardi::OutputFormat;
#[cfg(feature = "rayon")]
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

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

    /// DICOM field to extract, by its standard dictionary alias (e.g.
    /// PatientID, PatientName, StudyDate, Modality). Repeatable. Defaults to
    /// PatientID and PatientName.
    #[arg(short, long = "tag")]
    tags: Vec<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    path_exists(&args.path)?;

    let field_names = if args.tags.is_empty() {
        DEFAULT_FIELDS.iter().map(|s| s.to_string()).collect()
    } else {
        args.tags
    };
    let fields: Vec<&str> = field_names.iter().map(String::as_str).collect();

    let writer = StreamWriter::new(args.format, args.output.as_deref(), &field_names)?;

    #[cfg(feature = "rayon")]
    WalkDir::new(&args.path)
        .into_iter()
        .par_bridge()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| seek_record(&entry, &fields, args.debug))
        .for_each(|record| {
            if let Err(err) = writer.write_record(&record) {
                eprintln!("Error writing record {}: {}", record, err);
            }
        });

    #[cfg(not(feature = "rayon"))]
    WalkDir::new(&args.path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| seek_record(&entry, &fields, args.debug))
        .for_each(|record| {
            if let Err(err) = writer.write_record(&record) {
                eprintln!("Error writing record {}: {}", record, err);
            }
        });

    writer.finish()?;

    Ok(())
}

fn seek_record(entry: &DirEntry, fields: &[&str], debug: bool) -> Option<Record> {
    match parse(entry, fields) {
        Ok(record) => Some(record),
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
