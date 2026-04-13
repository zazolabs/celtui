//! Data persistence for saving and loading sight observations
//!
//! This module provides functionality to save sight observations to disk
//! and load them back, enabling users to work across multiple sessions.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Get the default data directory for saving sights
///
/// Uses platform-specific conventions:
/// - Linux/macOS: ~/.local/share/celtui
/// - Windows: %APPDATA%\celtui
pub fn get_data_dir() -> Result<PathBuf> {
    let data_dir = if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA")
            .context("Could not find APPDATA directory")?;
        PathBuf::from(appdata).join("celtui")
    } else {
        let home = std::env::var("HOME")
            .context("Could not find HOME directory")?;
        PathBuf::from(home).join(".local").join("share").join("celtui")
    };

    // Create directory if it doesn't exist
    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)
            .context("Failed to create data directory")?;
    }

    Ok(data_dir)
}

/// Save data to a JSON file
///
/// # Arguments
/// * `data` - The data to save (must implement Serialize)
/// * `filename` - Name of the file (will be saved in data directory)
///
/// # Returns
/// The full path where the file was saved
pub fn save_to_file<T: Serialize>(data: &T, filename: &str) -> Result<PathBuf> {
    let data_dir = get_data_dir()?;
    let file_path = data_dir.join(filename);

    let json = serde_json::to_string_pretty(data)
        .context("Failed to serialize data to JSON")?;

    fs::write(&file_path, json)
        .context(format!("Failed to write to file: {:?}", file_path))?;

    Ok(file_path)
}

/// Load data from a JSON file
///
/// # Arguments
/// * `filename` - Name of the file (will look in data directory)
///
/// # Returns
/// The deserialized data, or an error if the file doesn't exist or is invalid
pub fn load_from_file<T: for<'de> Deserialize<'de>>(filename: &str) -> Result<T> {
    let data_dir = get_data_dir()?;
    let file_path = data_dir.join(filename);

    let json = fs::read_to_string(&file_path)
        .context(format!("Failed to read file: {:?}", file_path))?;

    let data: T = serde_json::from_str(&json)
        .context("Failed to deserialize JSON data")?;

    Ok(data)
}

/// List all saved files in the data directory
pub fn list_saved_files(extension: &str) -> Result<Vec<String>> {
    let data_dir = get_data_dir()?;

    let mut files = Vec::new();
    if data_dir.exists() {
        for entry in fs::read_dir(&data_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == extension {
                    if let Some(filename) = path.file_name() {
                        if let Some(name) = filename.to_str() {
                            files.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestData {
        name: String,
        value: i32,
    }

    #[test]
    fn test_get_data_dir() {
        let dir = get_data_dir().unwrap();
        assert!(dir.exists());
        assert!(dir.is_dir());
    }

    #[test]
    fn test_save_and_load() {
        let test_data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        let filename = "test_save_load.json";

        // Save
        let path = save_to_file(&test_data, filename).unwrap();
        assert!(path.exists());

        // Load
        let loaded: TestData = load_from_file(filename).unwrap();
        assert_eq!(loaded, test_data);

        // Cleanup
        fs::remove_file(path).ok();
    }

    #[test]
    fn test_list_saved_files() {
        // Create some test files
        let test_data = TestData {
            name: "test".to_string(),
            value: 1,
        };

        let filename1 = "test_list_1.json";
        let filename2 = "test_list_2.json";

        save_to_file(&test_data, filename1).unwrap();
        save_to_file(&test_data, filename2).unwrap();

        // List files
        let files = list_saved_files("json").unwrap();
        assert!(files.contains(&filename1.to_string()));
        assert!(files.contains(&filename2.to_string()));

        // Cleanup
        let data_dir = get_data_dir().unwrap();
        fs::remove_file(data_dir.join(filename1)).ok();
        fs::remove_file(data_dir.join(filename2)).ok();
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result: Result<TestData> = load_from_file("nonexistent_file_xyz.json");
        assert!(result.is_err());
    }
}
