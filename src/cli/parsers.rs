//! Custom argument parsers for CLI.

/// Parse a KEY=VALUE string into a tuple.
pub fn parse_key_val(s: &str) -> Result<(String, String), String> {
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=VALUE: `{}`", s))?;
    Ok((s[..pos].to_string(), s[pos + 1..].to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_key_val_valid() {
        assert_eq!(
            parse_key_val("FOO=bar").unwrap(),
            ("FOO".to_string(), "bar".to_string())
        );
    }

    #[test]
    fn test_parse_key_val_empty_value() {
        assert_eq!(
            parse_key_val("FOO=").unwrap(),
            ("FOO".to_string(), "".to_string())
        );
    }

    #[test]
    fn test_parse_key_val_value_with_equals() {
        assert_eq!(
            parse_key_val("FOO=bar=baz").unwrap(),
            ("FOO".to_string(), "bar=baz".to_string())
        );
    }

    #[test]
    fn test_parse_key_val_invalid() {
        assert!(parse_key_val("FOOBAR").is_err());
    }
}
