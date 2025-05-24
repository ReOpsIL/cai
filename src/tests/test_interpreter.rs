#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_assignment() {
        let input = r#"x = "hello""#;
        let result = parser().parse(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_function_call() {
        let input = r#"result = read_file("test.txt")"#;
        let result = parser().parse(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_foreach() {
        let input = r#"
            foreach(items) {
                x = "test"
            }
        "#;
        let result = parser().parse(input);
        assert!(result.is_ok());
    }
}