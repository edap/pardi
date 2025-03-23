use anyhow::{anyhow, Context, Error, Result};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

pub fn print_error_messages(err: Error, debug_flag: bool) {
    if debug_flag {
        eprintln!("{}", err);
    }
}

pub fn print_catalog(content: String, output: &Option<PathBuf>) -> Result<()> {
    match output {
        Some(output_path) => {
            if let Err(err) = write_to_file(output_path, content) {
                return Err(anyhow!("File can't be created")).context(format!(
                    "Error writing to file {}: {}",
                    output_path.display(),
                    err,
                ));
            }
        }
        None => {
            println!("{}", content);
        }
    }
    Ok(())
}

fn write_to_file(output_path: &std::path::PathBuf, content: String) -> Result<()> {
    let mut file = File::create(output_path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    #[test]
    fn test_print_catalog_on_screen() {
        let content = "My Catalog".to_string();
        assert!(print_catalog(content.clone(), &None).is_ok());
    }

    #[test]
    fn test_print_catalog_on_file() {
        let content = "TMy Catalog".to_string();
        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("test.txt");
        assert!(print_catalog(content.clone(), &Some(output_path.clone())).is_ok());

        let mut file_content = String::new();
        File::open(&output_path)
            .and_then(|mut file| file.read_to_string(&mut file_content))
            .expect("Failed to read file content");
        assert_eq!(file_content, content);

        std::fs::remove_file(&output_path).expect("Failed to remove temporary file");
    }
}
