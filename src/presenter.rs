//! Streams parsed [`Record`]s to an output sink as CSV or JSON, one record
//! at a time.

use crate::parser::Record;
use crate::OutputFormat;
use anyhow::{Context, Result};
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::sync::Mutex;

/// The per-format writer state. Kept behind a single [`Mutex`] (inside
/// [`StreamWriter`]) so that whatever a format needs to track between
/// records — the CSV writer's internal buffer, or whether the next JSON
/// entry needs a leading comma — is protected by the same lock that
/// serializes writes, instead of a second synchronization primitive.
enum Sink {
    Csv(Box<csv::Writer<Box<dyn Write + Send>>>),
    Json {
        writer: Box<dyn Write + Send>,
        first: bool,
    },
}

/// Writes records to the output one at a time. Safe to call
/// [`write_record`](StreamWriter::write_record) from multiple threads
/// concurrently (e.g. from a rayon parallel iterator).
pub struct StreamWriter {
    sink: Mutex<Sink>,
}

impl StreamWriter {
    /// Opens `output` (or stdout, if `None`) and writes the format-specific
    /// preamble (a CSV header built from `field_names`, or a JSON `[`).
    pub fn new(
        format: OutputFormat,
        output: Option<&Path>,
        field_names: &[String],
    ) -> Result<Self> {
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
        writer: Box<dyn Write + Send>,
        field_names: &[String],
    ) -> Result<Self> {
        let sink = match format {
            OutputFormat::Csv => {
                let mut csv_writer = csv::WriterBuilder::new()
                    // Deterministic across platforms, and to keep it a
                    // simple diff from the previous plain-text output.
                    .terminator(csv::Terminator::Any(b'\n'))
                    .from_writer(writer);
                let mut header: Vec<&str> = field_names.iter().map(String::as_str).collect();
                header.push("File");
                csv_writer.write_record(header)?;
                Sink::Csv(Box::new(csv_writer))
            }
            OutputFormat::Json => {
                let mut writer = writer;
                write!(writer, "[")?;
                Sink::Json {
                    writer,
                    first: true,
                }
            }
        };
        Ok(Self {
            sink: Mutex::new(sink),
        })
    }

    /// Writes one record. For CSV, values are written in the same order as
    /// the `field_names` given to [`new`](StreamWriter::new)/
    /// [`with_writer`](StreamWriter::with_writer) — every record must have
    /// been parsed with that same field list.
    pub fn write_record(&self, record: &Record) -> Result<()> {
        // A panic while a worker holds this lock (e.g. from a downstream
        // I/O error turned into a panic elsewhere) shouldn't take every
        // other thread's writes down with it: the writer itself isn't left
        // in a broken state by that, so recovering the guard is safe.
        let mut sink = self.sink.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        match &mut *sink {
            Sink::Csv(writer) => {
                let file = record.file.display().to_string();
                let mut row: Vec<&str> = record.fields.iter().map(|(_, v)| v.as_str()).collect();
                row.push(&file);
                writer.write_record(row)?;
            }
            Sink::Json { writer, first } => {
                if *first {
                    writeln!(writer)?;
                    *first = false;
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
        let sink = self
            .sink
            .into_inner()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match sink {
            Sink::Csv(mut writer) => writer.flush()?,
            Sink::Json { mut writer, .. } => {
                writeln!(writer, "\n]")?;
                writer.flush()?;
            }
        }
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
                    // Contains a comma and a double quote: exercises CSV
                    // escaping (RFC 4180: wrap in quotes, double the quote).
                    ("PatientID".to_string(), "3".to_string()),
                    ("PatientName".to_string(), "Ballard, \"Jim\"".to_string()),
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
        let expected = "PatientID,PatientName,File\n1,Philip Dick,file1.dcm\n2,Kurt Vonnegut,file2.dcm\n3,\"Ballard, \"\"Jim\"\"\",file3.dcm\n";
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
        assert_eq!(records[2]["PatientName"], "Ballard, \"Jim\"");
        assert_eq!(records[0]["file"], "file1.dcm");
    }
}
