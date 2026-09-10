//! Streams parsed [`Record`]s to an output sink as CSV or JSON, one record
//! at a time.

use crate::parser::Record;
use crate::OutputFormat;
use anyhow::{Context, Result};
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// Writes records to the output one at a time. Safe to call
/// [`write_record`](StreamWriter::write_record) from multiple threads
/// concurrently (e.g. from a rayon parallel iterator).
pub struct StreamWriter {
    format: OutputFormat,
    writer: Mutex<Box<dyn Write + Send>>,
    first: AtomicBool,
}

impl StreamWriter {
    /// Opens `output` (or stdout, if `None`) and writes the format-specific
    /// preamble (a CSV header built from `field_names`, or a JSON `[`).
    pub fn new(format: OutputFormat, output: Option<&Path>, field_names: &[String]) -> Result<Self> {
        let writer: Box<dyn Write + Send> = match output {
            Some(path) => Box::new(
                File::create(path)
                    .with_context(|| format!("Error creating file {}", path.display()))?,
            ),
            None => Box::new(io::stdout()),
        };
        Self::with_writer(format, writer, field_names)
    }

    /// Builds a writer over an arbitrary sink (used for benchmarks/tests
    /// that don't want to write to stdout or a real file).
    pub fn with_writer(
        format: OutputFormat,
        mut writer: Box<dyn Write + Send>,
        field_names: &[String],
    ) -> Result<Self> {
        match format {
            OutputFormat::Json => write!(writer, "[")?,
            OutputFormat::Csv => writeln!(writer, "{},File", field_names.join(","))?,
        }
        Ok(Self {
            format,
            writer: Mutex::new(writer),
            first: AtomicBool::new(true),
        })
    }

    /// Writes one record. For CSV, values are written in the same order as
    /// the `field_names` given to [`new`](StreamWriter::new)/
    /// [`with_writer`](StreamWriter::with_writer) — every record must have
    /// been parsed with that same field list.
    pub fn write_record(&self, record: &Record) -> Result<()> {
        let mut writer = self.writer.lock().unwrap();
        match self.format {
            OutputFormat::Csv => {
                for (_, value) in &record.fields {
                    write!(writer, "{},", value)?;
                }
                writeln!(writer, "{}", record.file.display())?;
            }
            OutputFormat::Json => {
                if self.first.swap(false, Ordering::SeqCst) {
                    writeln!(writer)?;
                } else {
                    writeln!(writer, ",")?;
                }
                let json = serde_json::to_string_pretty(record)?;
                write!(writer, "{}", json)?;
            }
        }
        Ok(())
    }

    pub fn finish(self) -> Result<()> {
        let mut writer = self.writer.into_inner().unwrap();
        if let OutputFormat::Json = self.format {
            writeln!(writer, "\n]")?;
        }
        writer.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;

    #[derive(Clone)]
    struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedBuffer {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn field_names() -> Vec<String> {
        vec!["PatientID".to_string(), "PatientName".to_string()]
    }

    fn sample_records() -> Vec<Record> {
        vec![
            Record {
                file: PathBuf::from("file1.dcm"),
                fields: vec![
                    ("PatientID".to_string(), "1".to_string()),
                    ("PatientName".to_string(), "Philip Dick".to_string()),
                ],
            },
            Record {
                file: PathBuf::from("file2.dcm"),
                fields: vec![
                    ("PatientID".to_string(), "2".to_string()),
                    ("PatientName".to_string(), "Kurt Vonnegut".to_string()),
                ],
            },
            Record {
                file: PathBuf::from("file3.dcm"),
                fields: vec![
                    ("PatientID".to_string(), "3".to_string()),
                    ("PatientName".to_string(), "James Ballard".to_string()),
                ],
            },
        ]
    }

    fn run(format: OutputFormat) -> String {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let writer = StreamWriter::with_writer(
            format,
            Box::new(SharedBuffer(buffer.clone())),
            &field_names(),
        )
        .unwrap();
        for record in sample_records() {
            writer.write_record(&record).unwrap();
        }
        writer.finish().unwrap();

        let bytes = buffer.lock().unwrap().clone();
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn test_stream_as_csv() {
        let expected = "PatientID,PatientName,File\n1,Philip Dick,file1.dcm\n2,Kurt Vonnegut,file2.dcm\n3,James Ballard,file3.dcm\n";
        assert_eq!(run(OutputFormat::Csv), expected);
    }

    #[test]
    fn test_stream_as_json() {
        let result = run(OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        let records = parsed.as_array().unwrap();
        assert_eq!(records.len(), 3);
        assert_eq!(records[0]["PatientID"], "1");
        assert_eq!(records[0]["PatientName"], "Philip Dick");
        assert_eq!(records[2]["PatientID"], "3");
        assert_eq!(records[0]["file"], "file1.dcm");
    }
}
