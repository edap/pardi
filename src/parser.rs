use anyhow::{Context, Result};
use dicom::core::Tag;
use dicom::object::open_file;
use serde::{Deserialize, Serialize};
use std::{fmt, path::PathBuf};
use walkdir::DirEntry;

const GROUP_NUMBER: u16 = 0x0010;
const ELEMENT_NUMBER_PATIENT_NAME: u16 = 0x0010;
const ELEMENT_NUMBER_PATIENT_ID: u16 = 0x0020;

#[derive(Debug, Serialize, Deserialize)]
pub struct Patient {
    pub patient_id: String,
    pub patient_name: String,
    pub file: PathBuf,
}

impl fmt::Display for Patient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({}, {}, {})",
            self.patient_id, self.patient_name, self.file.display()
        )
    }
}

pub fn parse(entry: &DirEntry) -> Result<Patient> {
    let path = &entry.path().to_path_buf();
    let obj = open_file(path)
        .with_context(|| format!("The file: {} is not a valid DICOM file", path.display()))?;
    let patient_name = obj
        .element(Tag(GROUP_NUMBER, ELEMENT_NUMBER_PATIENT_NAME))
        .with_context(|| {
            format!(
                "Failed to find PatientName Tag for file: {}",
                path.display()
            )
        })?;
    let patient_id = obj
        .element(Tag(GROUP_NUMBER, ELEMENT_NUMBER_PATIENT_ID))
        .with_context(|| format!("Failed to find PatientID Tag for file: {}", path.display()))?;
    let patient_name_str = patient_name.to_str().with_context(|| {
        format!(
            "Failed to convert PatientName tag to string in file: {}",
            path.display()
        )
    })?;
    let patient_id_str = patient_id.to_str().with_context(|| {
        format!(
            "Failed to convert PatientId tag to string in file: {}",
            path.display()
        )
    })?;

    Ok(Patient {
        patient_id: patient_id_str.to_string(),
        patient_name: patient_name_str.to_string(),
        file: path.to_path_buf(),
    })
}

#[cfg(test)]
mod test {
    use crate::parser;
    use std::path::PathBuf;
    use walkdir::WalkDir;

    #[test]
    fn test_parse_valid_dicom_file() {
        let dir_path = PathBuf::from("test/0020.DCM");
        for entry in WalkDir::new(dir_path).into_iter().filter_map(|e| e.ok()) {
            let result = parser::parse(&entry);
            assert!(result.is_ok());
            let patient = result.unwrap();
            assert_eq!(patient.patient_id, "123-45-6789");
            assert_eq!(patient.patient_name, "Rubo DEMO");
            assert_eq!(&patient.file, &entry.path().to_path_buf());
        }
    }

    #[test]
    fn test_parse_invalid_dicom_file() {
        let dir_path = PathBuf::from("file.txt");
        for entry in WalkDir::new(dir_path).into_iter().filter_map(|e| e.ok()) {
            let result = parser::parse(&entry);
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
            let result = parser::parse(&entry);
            assert!(result.is_err());
            let err_msg = result.unwrap_err().to_string();
            assert!(err_msg.contains("The file:"));
            assert!(err_msg.contains(&entry.path().display().to_string()));
            assert!(err_msg.contains("is not a valid DICOM file"));
        }
    }
}
