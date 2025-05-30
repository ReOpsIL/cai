use glob::glob;
use std::fs;
use std::collections::HashMap;

pub struct FileService;

impl FileService {
    pub fn new() -> Self {
        Self
    }

    pub fn list_files(&self, pattern: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut files = Vec::new();
        for entry in glob(pattern)? {
            match entry {
                Ok(path) => {
                    if path.is_file() {
                        files.push(path.display().to_string());
                    }
                }
                Err(e) => println!("{:?}", e),
            }
        }
        Ok(files)
    }

    pub fn list_folders(&self, pattern: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut folders = Vec::new();
        for entry in glob(pattern)? {
            match entry {
                Ok(path) => {
                    if path.is_dir() {
                        folders.push(path.display().to_string());
                    }
                }
                Err(e) => println!("{:?}", e),
            }
        }
        Ok(folders)
    }

    pub fn read_file(&self, filename: &str) -> Result<String, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(filename)?;
        Ok(contents)
    }

    pub fn write_file(&self, filename: &str, content: &str) -> Result<(), Box<dyn std::error::Error>> {
        fs::write(filename, content)?;
        Ok(())
    }

    pub fn read_files(&self, pattern: &str) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let mut file_contents = HashMap::new();
        for entry in glob(pattern)? {
            match entry {
                Ok(path) => {
                    if path.is_file() {
                        let filename = path.display().to_string();
                        let contents = fs::read_to_string(&filename)?;
                        file_contents.insert(filename, contents);
                    }
                }
                Err(e) => println!("{:?}", e),
            }
        }
        Ok(file_contents)
    }

    pub fn read_folder(&self, pattern: &str) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let mut folder_contents = HashMap::new();
        for entry in glob(pattern)? {
            match entry {
                Ok(path) => {
                    if path.is_dir() {
                        for file_entry in fs::read_dir(path)? {
                            let file_path = file_entry?.path();
                            if file_path.is_file() {
                                let filename = file_path.display().to_string();
                                let contents = fs::read_to_string(&filename)?;
                                folder_contents.insert(filename, contents);
                            }
                        }
                    }
                }
                Err(e) => println!("{:?}", e),
            }
        }
        Ok(folder_contents)
    }

    pub fn create_file(&self, path: &str, content: &str) -> Result<(), Box<dyn std::error::Error>> {
        fs::write(path, content)?;
        Ok(())
    }

    pub fn delete_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        fs::remove_file(path)?;
        Ok(())
    }

    pub fn create_directory(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(path)?;
        Ok(())
    }

    pub fn delete_directory(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        fs::remove_dir_all(path)?;
        Ok(())
    }
}