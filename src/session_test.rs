#[cfg(test)]
mod tests {
    use crate::session::*;
    use std::fs;

    #[test]
    fn test_public_session_api() {
        // Test the public API functions
        let result = create_session("test-api-session");
        assert!(result.is_ok());
        
        let sessions_list = list_sessions().unwrap();
        assert!(sessions_list.contains("test-api-session"));
        
        let switch_result = switch_session("test-api-session");
        assert!(switch_result.is_ok());
        
        let current_info = get_current_session_info().unwrap();
        assert!(current_info.contains("test-api-session"));
    }
}