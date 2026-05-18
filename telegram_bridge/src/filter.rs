use regex::Regex;

const SENSITIVE_PATTERNS: &[(&str, &str)] = &[
    (r"(?i)(bot_token|api_key|secret|password)\s*[=:]\s*\S+", "***"),
    (r"sk-[a-zA-Z0-9]{20,}", "***sk-...***"),
    (r"ghp_[a-zA-Z0-9]{36}", "***ghp_...***"),
    (r"gho_[a-zA-Z0-9]{36}", "***gho_...***"),
];

pub fn filter_sensitive(input: &str) -> String {
    let mut output = input.to_string();
    for (pattern, replacement) in SENSITIVE_PATTERNS {
        if let Ok(re) = Regex::new(pattern) {
            output = re.replace_all(&output, *replacement).to_string();
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_openai_key() {
        let input = "my key is sk-abc123def456ghi789jkl012";
        let result = filter_sensitive(input);
        assert!(!result.contains("sk-abc123def456ghi"));
        assert!(result.contains("***sk-...***"));
    }

    #[test]
    fn test_filter_github_pat() {
        let input = "token ghp_abcdef123456789012345678901234567890";
        let result = filter_sensitive(input);
        assert!(!result.contains("ghp_abcdef"));
        assert!(result.contains("***ghp_...***"));
    }

    #[test]
    fn test_no_false_positive() {
        let input = "normal text without any secrets";
        let result = filter_sensitive(input);
        assert_eq!(result, input);
    }
}
