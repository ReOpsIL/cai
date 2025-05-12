#[cfg(test)]
mod tests {
    use crate::chat;
    use crate::commands;
    use crate::commands_registry;
    use crate::files::files;
    use regex::Regex;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_files(temp_dir: &TempDir) -> Vec<String> {
        let base_path = temp_dir.path();

        let file_paths = vec![
            base_path.join("test1.txt"),
            base_path.join("test2.txt"),
            base_path.join("other.txt"),
        ];

        // Create files with content
        File::create(&file_paths[0])
            .unwrap()
            .write_all(b"Content of test1")
            .unwrap();
        File::create(&file_paths[1])
            .unwrap()
            .write_all(b"Content of test2")
            .unwrap();
        File::create(&file_paths[2])
            .unwrap()
            .write_all(b"Content of other file")
            .unwrap();

        // Return string paths
        file_paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect()
    }

    #[test]
    fn test_read_files_function() {
        // Create temporary directory and test files
        let temp_dir = TempDir::new().unwrap();
        let file_paths = create_test_files(&temp_dir);

        // Get the pattern for test*.txt files
        let pattern = format!("{}/test*.txt", temp_dir.path().to_string_lossy());

        // Call the read_files function
        let result = files::read_files(&pattern).unwrap();

        // Verify we got the expected files (only test1.txt and test2.txt, not other.txt)
        assert_eq!(result.len(), 2);
        assert!(result.contains_key(&file_paths[0]));
        assert!(result.contains_key(&file_paths[1]));
        assert!(!result.contains_key(&file_paths[2]));

        // Verify content
        assert_eq!(result.get(&file_paths[0]).unwrap(), "Content of test1");
        assert_eq!(result.get(&file_paths[1]).unwrap(), "Content of test2");
    }

    #[test]
    fn test_read_files_command() {
        // Register commands
        commands::register_all_commands();

        // Create temporary directory and test files
        let temp_dir = TempDir::new().unwrap();
        create_test_files(&temp_dir);

        // Get the pattern for test*.txt files
        let pattern = format!("{}/test*.txt", temp_dir.path().to_string_lossy());

        // Create the command string
        let command = format!("@read-files(test-mem, {})", pattern);

        // Execute the command
        let result = commands_registry::execute_command(&command).unwrap();

        // Verify result message
        assert!(result.is_some());
        let message = result.unwrap();
        assert!(message.contains("Files matching pattern"));
        assert!(message.contains("test-mem"));

        // Verify memory content
        let memory = chat::get_memory().lock().unwrap();
        assert!(memory.contains_key("test-mem"));

        let content = memory.get("test-mem").unwrap();
        assert!(content.contains("Content of test1"));
        assert!(content.contains("Content of test2"));
        assert!(content.contains("File:"));

        // Verify other.txt content is not included
        assert!(!content.contains("Content of other file"));
    }

    #[test]
    fn test_read_files_with_no_matches() {
        // Register commands
        commands::register_all_commands();

        // Create temporary directory
        let temp_dir = TempDir::new().unwrap();

        // Get the pattern for non-existent files
        let pattern = format!("{}/nonexistent*.txt", temp_dir.path().to_string_lossy());

        // Create the command string
        let command = format!("@read-files(empty-mem, {})", pattern);

        // Execute the command
        let result = commands_registry::execute_command(&command).unwrap();

        // Verify result message
        assert!(result.is_some());

        // Verify memory content - should be empty or contain an empty map
        let memory = chat::get_memory().lock().unwrap();
        assert!(memory.contains_key("empty-mem"));

        let content = memory.get("empty-mem").unwrap();
        assert!(content.is_empty() || content == "{}");
    }

    #[test]
    fn test_command_regex_patterns() {
        // Test the regex pattern for read-files command
        let read_files_pattern = Regex::new(r"@read-files\(\s*(\S+)\s*,\s*(\S+)\s*\)").unwrap();

        // Test with various valid formats
        let test_cases = vec![
            "@read-files(memory-id, wildcard.txt)",
            "@read-files(  memory-id  ,  wildcard.txt  )",
            "@read-files(memory-id,wildcard.txt)",
        ];

        for test_case in test_cases {
            let captures = read_files_pattern.captures(test_case).unwrap();
            assert_eq!(captures.get(1).unwrap().as_str(), "memory-id");
            assert_eq!(captures.get(2).unwrap().as_str(), "wildcard.txt");
        }

        // Register commands
        commands::register_all_commands();

        // Test the command registry parsing
        let command_result =
            commands_registry::parse_command("@read-files(memory-id, wildcard.txt)").unwrap();
        assert_eq!(command_result.command_name, "read-files");
        assert_eq!(command_result.parameters.len(), 2);
        assert_eq!(command_result.parameters[0], "memory-id");
        assert_eq!(command_result.parameters[1], "wildcard.txt");
    }
}
