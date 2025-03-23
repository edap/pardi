use crate::parser;

pub fn as_csv(patients: &[parser::Patient]) -> String {
    let header = "Patient ID,Patient Name,File";
    let rows: Vec<String> = patients
        .iter()
        .map(|patient| {
            format!(
                "{},{},{}",
                patient.patient_id,
                patient.patient_name,
                patient.file.display().to_string()
            )
        })
        .collect();

    let csv_string = format!("{}\n{}", header, rows.join("\n"));
    csv_string
}

pub fn as_json(patients: &[parser::Patient]) -> String {
    serde_json::to_string_pretty(patients).unwrap_or_default()
}

#[cfg(test)]
mod test {
    use crate::parser;
    use crate::presenter;
    use std::path::PathBuf;

    #[test]
    fn test_as_csv() {
        let patients = vec![
            parser::Patient {
                patient_id: "1".to_string(),
                patient_name: "Philip Dick".to_string(),
                file: PathBuf::from("file1.dcm"),
            },
            parser::Patient {
                patient_id: "2".to_string(),
                patient_name: "Kurt Vonnegut".to_string(),
                file: PathBuf::from("file2.dcm"),
            },
            parser::Patient {
                patient_id: "3".to_string(),
                patient_name: "James Ballard".to_string(),
                file: PathBuf::from("file3.dcm"),
            },
        ];

        let expected_result =
        "Patient ID,Patient Name,File\n1,Philip Dick,file1.dcm\n2,Kurt Vonnegut,file2.dcm\n3,James Ballard,file3.dcm";

        assert_eq!(presenter::as_csv(&patients), expected_result);
    }

    #[test]
    fn test_as_json() {
        let patients = vec![
            parser::Patient {
                patient_id: "1".to_string(),
                patient_name: "Philip Dick".to_string(),
                file: PathBuf::from("file1.dcm"),
            },
            parser::Patient {
                patient_id: "2".to_string(),
                patient_name: "Kurt Vonnegut".to_string(),
                file: PathBuf::from("file2.dcm"),
            },
            parser::Patient {
                patient_id: "3".to_string(),
                patient_name: "James Ballard".to_string(),
                file: PathBuf::from("file3.dcm"),
            },
        ];

        let expected_result = r#"[{
    "patient_id": "1",
    "patient_name": "Philip Dick",
    "file": "file1.dcm"
    }, {
    "patient_id": "2",
    "patient_name": "Kurt Vonnegut",
    "file": "file2.dcm"
    }, {
    "patient_id": "3",
    "patient_name": "James Ballard",
    "file": "file3.dcm"
    }]"#;

        let actual_result = presenter::as_json(&patients);

        // Normalize whitespace for comparison
        let expected_result_normalized = expected_result.replace("\n", "").replace(" ", "");
        let actual_result_normalized = actual_result.replace("\n", "").replace(" ", "");

        assert_eq!(actual_result_normalized, expected_result_normalized);
    }
}
