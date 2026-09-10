use crate::parser::Patient;
use crate::OutputFormat;
use anyhow::{Context, Result};
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// Writes patients to the output one at a time, instead of buffering them
/// all in memory before printing.
pub struct StreamWriter {
    format: OutputFormat,
    writer: Mutex<Box<dyn Write + Send>>,
    first: AtomicBool,
}

impl StreamWriter {
    pub fn new(format: OutputFormat, output: Option<&Path>) -> Result<Self> {
        let writer: Box<dyn Write + Send> = match output {
            Some(path) => Box::new(
                File::create(path)
                    .with_context(|| format!("Error creating file {}", path.display()))?,
            ),
            None => Box::new(io::stdout()),
        };
        Self::with_writer(format, writer)
    }

    /// Builds a writer over an arbitrary sink (used for benchmarks/tests
    /// that don't want to write to stdout or a real file).
    pub fn with_writer(format: OutputFormat, mut writer: Box<dyn Write + Send>) -> Result<Self> {
        match format {
            OutputFormat::Json => write!(writer, "[")?,
            OutputFormat::Csv => writeln!(writer, "Patient ID,Patient Name,File")?,
        }
        Ok(Self {
            format,
            writer: Mutex::new(writer),
            first: AtomicBool::new(true),
        })
    }

    pub fn write_patient(&self, patient: &Patient) -> Result<()> {
        let mut writer = self.writer.lock().unwrap();
        match self.format {
            OutputFormat::Csv => {
                writeln!(
                    writer,
                    "{},{},{}",
                    patient.patient_id,
                    patient.patient_name,
                    patient.file.display()
                )?;
            }
            OutputFormat::Json => {
                if self.first.swap(false, Ordering::SeqCst) {
                    writeln!(writer)?;
                } else {
                    writeln!(writer, ",")?;
                }
                let json = serde_json::to_string_pretty(patient)?;
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

    fn sample_patients() -> Vec<Patient> {
        vec![
            Patient {
                patient_id: "1".to_string(),
                patient_name: "Philip Dick".to_string(),
                file: PathBuf::from("file1.dcm"),
            },
            Patient {
                patient_id: "2".to_string(),
                patient_name: "Kurt Vonnegut".to_string(),
                file: PathBuf::from("file2.dcm"),
            },
            Patient {
                patient_id: "3".to_string(),
                patient_name: "James Ballard".to_string(),
                file: PathBuf::from("file3.dcm"),
            },
        ]
    }

    fn run(format: OutputFormat) -> String {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let writer =
            StreamWriter::with_writer(format, Box::new(SharedBuffer(buffer.clone()))).unwrap();
        for patient in sample_patients() {
            writer.write_patient(&patient).unwrap();
        }
        writer.finish().unwrap();

        let bytes = buffer.lock().unwrap().clone();
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn test_stream_as_csv() {
        let expected = "Patient ID,Patient Name,File\n1,Philip Dick,file1.dcm\n2,Kurt Vonnegut,file2.dcm\n3,James Ballard,file3.dcm\n";
        assert_eq!(run(OutputFormat::Csv), expected);
    }

    #[test]
    fn test_stream_as_json() {
        let result = run(OutputFormat::Json);
        let parsed: Vec<Patient> = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].patient_id, "1");
        assert_eq!(parsed[0].patient_name, "Philip Dick");
        assert_eq!(parsed[2].patient_id, "3");
    }
}
