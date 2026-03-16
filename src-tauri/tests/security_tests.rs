// WARNING: These tests validate a LOCAL COPY of redaction logic.
// Changes to transport_layer::TransportLayer::redact_env_value() are NOT covered
// by these tests because that function is a private method on a private impl block
// and cannot be imported from an integration test.
//
// If the sensitive-pattern list, threshold, or format string in
// transport_layer.rs changes, update the `redact_test_value` helper below to
// match, and add a note here explaining the divergence.
//
// TODO: Once `redact_env_value` is exposed (e.g. as a pub(crate) free function
// in a dedicated redaction module), replace `redact_test_value` with a direct
// import and remove this warning.

/// Security Tests for TurboMCP Studio Backend
///
/// Tests credential redaction, authentication, and other security features
#[cfg(test)]
mod credential_redaction_tests {
    use super::redact_test_value;

    /// Test credential redaction function (isolated unit test)
    ///
    /// This test verifies that the redact_env_value logic properly redacts
    /// sensitive environment variable values based on key patterns.
    #[test]
    fn test_redact_sensitive_api_key() {
        let key = "OPENAI_API_KEY";
        let value = "sk-proj-abc123xyz789";

        let redacted = redact_test_value(key, value);

        // Should show first 4 chars + redacted marker
        assert!(redacted.contains("sk-p"));
        assert!(redacted.contains("REDACTED"));
        assert!(
            !redacted.contains("abc123"),
            "Original value should not be visible"
        );
        assert!(
            !redacted.contains("xyz789"),
            "Original value should not be visible"
        );
    }

    #[test]
    fn test_redact_short_password() {
        let key = "PASSWORD";
        let value = "abc";

        let redacted = redact_test_value(key, value);

        // Short values (<=4 chars) should be fully redacted
        assert_eq!(redacted, "<REDACTED>");
        assert!(
            !redacted.contains("abc"),
            "Short password should be fully hidden"
        );
    }

    #[test]
    fn test_no_redaction_for_non_sensitive_keys() {
        let key = "DEBUG_MODE";
        let value = "true";

        let redacted = redact_test_value(key, value);

        // Non-sensitive values should not be redacted
        assert_eq!(redacted, "true");
    }

    #[test]
    fn test_redact_various_sensitive_patterns() {
        let test_cases = vec![
            ("API_KEY", "secret123"),
            ("AUTH_TOKEN", "bearer_token_xyz"),
            ("DATABASE_PASSWORD", "dbpass12345"),
            ("SECRET_ACCESS_KEY", "aws_secret_key"),
            ("BEARER_TOKEN", "jwt_token_here"),
        ];

        for (key, value) in test_cases {
            let redacted = redact_test_value(key, value);
            assert!(
                redacted.contains("REDACTED"),
                "Key {} should be redacted",
                key
            );
            // Ensure most of the original value is hidden
            assert!(
                value.len() > 4 && !redacted.contains(&value[4..]),
                "Key {} value should be partially hidden",
                key
            );
        }
    }

    #[test]
    fn test_case_insensitive_pattern_matching() {
        let test_cases = vec![
            ("api_key", "secret123"),
            ("Api_Token", "token456"),
            ("DATABASE_PASSWORD", "pass789"),
        ];

        for (key, value) in test_cases {
            let redacted = redact_test_value(key, value);
            assert!(
                redacted.contains("REDACTED"),
                "Case-insensitive matching should work for key: {}",
                key
            );
        }
    }

    #[test]
    fn test_empty_value_handling() {
        let key = "API_KEY";
        let value = "";

        let redacted = redact_test_value(key, value);

        // Empty values should be fully redacted
        assert_eq!(redacted, "<REDACTED>");
    }
}

#[cfg(test)]
mod log_redaction_integration_tests {
    use super::redact_test_value;

    /// Integration test to verify redaction appears in actual logs
    ///
    /// Note: This test verifies the concept. In practice, you'd want to
    /// capture log output and verify redaction appears there.
    #[test]
    fn test_log_redaction_concept() {
        // This test verifies the redaction behavior at a conceptual level
        // In a real integration test, we'd:
        // 1. Capture log output
        // 2. Trigger MCP connection with sensitive env vars
        // 3. Verify logs show redacted values

        let test_env_vars = vec![
            ("OPENAI_API_KEY", "sk-proj-test123"),
            ("DATABASE_PASSWORD", "postgresql_pass_12345"),
        ];

        for (key, value) in test_env_vars {
            let redacted = redact_test_value(key, value);

            // Verify basic redaction properties
            assert!(
                !redacted.contains(&value[4..]),
                "Log redaction should hide most of the value for {}",
                key
            );
        }
    }
}

/// Local copy of the redaction logic from `transport_layer::TransportLayer::redact_env_value`.
///
/// WARNING: This function MUST be kept in sync with `TransportLayer::redact_env_value`
/// in `src/mcp_client/transport_layer.rs`. If that function changes, update this one.
///
/// The function is a private method on a private impl block and cannot be directly
/// imported in integration tests.
fn redact_test_value(key: &str, value: &str) -> String {
    let sensitive_patterns = [
        "KEY",
        "TOKEN",
        "SECRET",
        "PASSWORD",
        "PASS",
        "PWD",
        "AUTH",
        "CREDENTIAL",
        "API",
        "BEARER",
        "ACCESS",
    ];

    let key_upper = key.to_uppercase();
    let is_sensitive = sensitive_patterns
        .iter()
        .any(|pattern| key_upper.contains(pattern));

    if is_sensitive {
        if value.len() <= 4 {
            "<REDACTED>".to_string()
        } else {
            let prefix = &value[..4.min(value.len())];
            format!("{}****<REDACTED>", prefix)
        }
    } else {
        value.to_string()
    }
}
