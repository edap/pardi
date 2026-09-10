//! Extracts a configurable set of DICOM fields out of a file.
//!
//! [`parse`] takes the
//! list of DICOM field names to extract (e.g. `"PatientID"`,
//! `"PatientName"`, `"StudyDate"`, `"Modality"`...). Any name known to the
//! [DICOM standard data dictionary][dicom::dictionary_std] works; see
//! <https://docs.rs/dicom-dictionary-std> for the full list of aliases, or
//! run a DICOM dump tool against a sample file to see which attributes it
//! carries.

use anyhow::{Context, Result};
use dicom::object::open_file;
use serde::ser::SerializeMap;
use serde::Serialize;
use std::{fmt, path::PathBuf};
use walkdir::DirEntry;

/// The fields extracted when the user does not ask for anything specific.
pub const DEFAULT_FIELDS: &[&str] = &["PatientID", "PatientName"];

/// One parsed DICOM file: the fields that were requested, in request order,
/// plus the file they came from.
#[derive(Debug)]
pub struct Record {
    pub file: PathBuf,
    pub fields: Vec<(String, String)>,
}

impl Record {
    /// Looks up an extracted field by name.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(field_name, _)| field_name == name)
            .map(|(_, value)| value.as_str())
    }
}

impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;
        for (name, value) in &self.fields {
            write!(f, "{}: {}, ", name, value)?;
        }
        write!(f, "{})", self.file.display())
    }
}

// Serialized as a flat JSON object so that arbitrary, user-chosen field
// names become the JSON keys, instead of being tied to fixed struct fields.
impl Serialize for Record {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.fields.len() + 1))?;
        for (name, value) in &self.fields {
            map.serialize_entry(name, value)?;
        }
        map.serialize_entry("file", &self.file)?;
        map.end()
    }
}

/// Opens `entry` as a DICOM file and extracts `fields` from it, by their
/// DICOM dictionary alias (e.g. `"PatientID"`, `"StudyDate"`).
///
/// Fails if the file is not a valid DICOM file, if one of the requested
/// fields is not present, or if a present field can't be read as text.
pub fn parse(entry: &DirEntry, fields: &[&str]) -> Result<Record> {
    let path = entry.path().to_path_buf();
    let obj = open_file(&path)
        .with_context(|| format!("The file: {} is not a valid DICOM file", path.display()))?;

    let mut extracted = Vec::with_capacity(fields.len());
    for &name in fields {
        let element = obj
            .element_by_name(name)
            .with_context(|| format!("Failed to find {} Tag for file: {}", name, path.display()))?;
        let value = element.to_str().with_context(|| {
            format!(
                "Failed to convert {} tag to string in file: {}",
                name,
                path.display()
            )
        })?;
        extracted.push((name.to_string(), value.into_owned()));
    }

    Ok(Record {
        file: path,
        fields: extracted,
    })
}

#[cfg(test)]
mod test {
    use crate::parser::{self, DEFAULT_FIELDS};
    use std::path::PathBuf;
    use walkdir::WalkDir;

    #[test]
    fn test_parse_valid_dicom_file() {
        let dir_path = PathBuf::from("test/0020.DCM");
        for entry in WalkDir::new(dir_path).into_iter().filter_map(|e| e.ok()) {
            let result = parser::parse(&entry, DEFAULT_FIELDS);
            assert!(result.is_ok());
            let record = result.unwrap();
            assert_eq!(record.get("PatientID"), Some("123-45-6789"));
            assert_eq!(record.get("PatientName"), Some("Rubo DEMO"));
            assert_eq!(&record.file, &entry.path().to_path_buf());
        }
    }

    #[test]
    fn test_parse_invalid_dicom_file() {
        let dir_path = PathBuf::from("file.txt");
        for entry in WalkDir::new(dir_path).into_iter().filter_map(|e| e.ok()) {
            let result = parser::parse(&entry, DEFAULT_FIELDS);
            assert!(result.is_err());
            let err_msg = result.unwrap_err().to_string();
            assert!(err_msg.contains("The file:"));
            assert!(err_msg.contains(&entry.path().display().to_string()));
            assert!(err_msg.contains("is not a valid DICOM file"));
        }
    }
    #[test]
    fn test_corrupted_dicom_file() {
        let dir_path = PathBuf::from("corrupted");
        for entry in WalkDir::new(dir_path).into_iter().filter_map(|e| e.ok()) {
            let result = parser::parse(&entry, DEFAULT_FIELDS);
            assert!(result.is_err());
            let err_msg = result.unwrap_err().to_string();
            assert!(err_msg.contains("The file:"));
            assert!(err_msg.contains(&entry.path().display().to_string()));
            assert!(err_msg.contains("is not a valid DICOM file"));
        }
    }

    #[test]
    fn test_parse_custom_field() {
        let dir_path = PathBuf::from("test/0020.DCM");
        for entry in WalkDir::new(dir_path).into_iter().filter_map(|e| e.ok()) {
            let result = parser::parse(&entry, &["PatientID"]);
            let record = result.unwrap();
            assert_eq!(record.fields, vec![("PatientID".to_string(), "123-45-6789".to_string())]);
            assert_eq!(record.get("PatientName"), None);
        }
    }
}
