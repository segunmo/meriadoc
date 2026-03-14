use std::collections::HashMap;

use crate::core::execution::ResolvedEnv;

/// Interpolates `${VAR}` patterns in strings using the provided environment.
///
/// Supports:
/// - `${VAR}` - Replaced with the value of VAR from the environment
/// - `${VAR:-default}` - Replaced with VAR's value, or "default" if VAR is not set
/// - `$$` - Escaped to a single `$`
///
/// Unknown variables without defaults are left as empty strings.
pub struct Interpolator;

impl Interpolator {
    /// Interpolate a single string
    pub fn interpolate(input: &str, env: &ResolvedEnv) -> String {
        Self::interpolate_with_special(input, env, &HashMap::new())
    }

    /// Interpolate with additional special variables (like PROJECT_ROOT)
    pub fn interpolate_with_special(
        input: &str,
        env: &ResolvedEnv,
        special: &HashMap<String, String>,
    ) -> String {
        let mut result = String::with_capacity(input.len());
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '$' {
                match chars.peek() {
                    // Escaped dollar sign: $$
                    Some('$') => {
                        chars.next();
                        result.push('$');
                    }
                    // Variable reference: ${VAR} or ${VAR:-default}
                    Some('{') => {
                        chars.next(); // consume '{'
                        let (var_name, default_value) = Self::parse_variable(&mut chars);

                        // Look up in special vars first, then env
                        let value = special
                            .get(&var_name)
                            .or_else(|| env.get(&var_name))
                            .map(|s| s.as_str());

                        match (value, default_value) {
                            (Some(v), _) => result.push_str(v),
                            (None, Some(d)) => result.push_str(&d),
                            (None, None) => {} // Empty string for undefined vars
                        }
                    }
                    // Plain $VAR (simple variable, no braces)
                    Some(c) if c.is_ascii_alphabetic() || *c == '_' => {
                        let var_name = Self::parse_simple_variable(&mut chars);
                        let value = special
                            .get(&var_name)
                            .or_else(|| env.get(&var_name))
                            .map(|s| s.as_str())
                            .unwrap_or("");
                        result.push_str(value);
                    }
                    // Standalone $ or $ followed by something else
                    _ => {
                        result.push('$');
                    }
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Parse a ${VAR} or ${VAR:-default} pattern
    /// Returns (var_name, optional_default)
    fn parse_variable(
        chars: &mut std::iter::Peekable<std::str::Chars>,
    ) -> (String, Option<String>) {
        let mut var_name = String::new();
        let mut default_value: Option<String> = None;
        let mut in_default = false;
        let mut default_str = String::new();

        while let Some(&c) = chars.peek() {
            if c == '}' {
                chars.next(); // consume '}'
                break;
            }

            chars.next(); // consume the character

            if in_default {
                default_str.push(c);
            } else if c == ':' {
                // Check for :- pattern
                if chars.peek() == Some(&'-') {
                    chars.next(); // consume '-'
                    in_default = true;
                } else {
                    var_name.push(c);
                }
            } else {
                var_name.push(c);
            }
        }

        if in_default {
            default_value = Some(default_str);
        }

        (var_name, default_value)
    }

    /// Parse a simple $VAR pattern (no braces)
    fn parse_simple_variable(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
        let mut var_name = String::new();

        while let Some(&c) = chars.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                var_name.push(c);
                chars.next();
            } else {
                break;
            }
        }

        var_name
    }

    /// Interpolate multiple strings
    #[cfg(test)]
    pub fn interpolate_all(inputs: &[String], env: &ResolvedEnv) -> Vec<String> {
        inputs.iter().map(|s| Self::interpolate(s, env)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_env(pairs: &[(&str, &str)]) -> ResolvedEnv {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn test_no_variables() {
        let env = make_env(&[]);
        assert_eq!(
            Interpolator::interpolate("hello world", &env),
            "hello world"
        );
    }

    #[test]
    fn test_simple_variable() {
        let env = make_env(&[("NAME", "Alice")]);
        assert_eq!(
            Interpolator::interpolate("Hello ${NAME}!", &env),
            "Hello Alice!"
        );
    }

    #[test]
    fn test_simple_variable_no_braces() {
        let env = make_env(&[("NAME", "Alice")]);
        assert_eq!(
            Interpolator::interpolate("Hello $NAME!", &env),
            "Hello Alice!"
        );
    }

    #[test]
    fn test_multiple_variables() {
        let env = make_env(&[("FIRST", "Hello"), ("SECOND", "World")]);
        assert_eq!(
            Interpolator::interpolate("${FIRST} ${SECOND}!", &env),
            "Hello World!"
        );
    }

    #[test]
    fn test_undefined_variable_empty() {
        let env = make_env(&[]);
        assert_eq!(Interpolator::interpolate("Hello ${NAME}!", &env), "Hello !");
    }

    #[test]
    fn test_default_value_when_undefined() {
        let env = make_env(&[]);
        assert_eq!(
            Interpolator::interpolate("Hello ${NAME:-World}!", &env),
            "Hello World!"
        );
    }

    #[test]
    fn test_default_value_not_used_when_defined() {
        let env = make_env(&[("NAME", "Alice")]);
        assert_eq!(
            Interpolator::interpolate("Hello ${NAME:-World}!", &env),
            "Hello Alice!"
        );
    }

    #[test]
    fn test_empty_default_value() {
        let env = make_env(&[]);
        assert_eq!(
            Interpolator::interpolate("Hello ${NAME:-}!", &env),
            "Hello !"
        );
    }

    #[test]
    fn test_escaped_dollar() {
        let env = make_env(&[("NAME", "Alice")]);
        assert_eq!(Interpolator::interpolate("Cost: $$100", &env), "Cost: $100");
    }

    #[test]
    fn test_dollar_not_followed_by_var() {
        let env = make_env(&[]);
        assert_eq!(Interpolator::interpolate("$5 bill", &env), "$5 bill");
    }

    #[test]
    fn test_special_variables() {
        let env = make_env(&[("NAME", "Alice")]);
        let special = [("PROJECT_ROOT".to_string(), "/home/user/project".to_string())]
            .into_iter()
            .collect();

        assert_eq!(
            Interpolator::interpolate_with_special(
                "Root: ${PROJECT_ROOT}, Name: ${NAME}",
                &env,
                &special
            ),
            "Root: /home/user/project, Name: Alice"
        );
    }

    #[test]
    fn test_special_overrides_env() {
        let env = make_env(&[("VAR", "from_env")]);
        let special = [("VAR".to_string(), "from_special".to_string())]
            .into_iter()
            .collect();

        assert_eq!(
            Interpolator::interpolate_with_special("${VAR}", &env, &special),
            "from_special"
        );
    }

    #[test]
    fn test_interpolate_all() {
        let env = make_env(&[("NAME", "World")]);
        let inputs = vec!["Hello ${NAME}".to_string(), "Goodbye ${NAME}".to_string()];
        let result = Interpolator::interpolate_all(&inputs, &env);
        assert_eq!(result, vec!["Hello World", "Goodbye World"]);
    }

    #[test]
    fn test_variable_at_end() {
        let env = make_env(&[("NAME", "test")]);
        assert_eq!(
            Interpolator::interpolate("value=${NAME}", &env),
            "value=test"
        );
    }

    #[test]
    fn test_adjacent_variables() {
        let env = make_env(&[("A", "foo"), ("B", "bar")]);
        assert_eq!(Interpolator::interpolate("${A}${B}", &env), "foobar");
    }

    #[test]
    fn test_underscore_in_variable_name() {
        let env = make_env(&[("MY_VAR", "value")]);
        assert_eq!(Interpolator::interpolate("${MY_VAR}", &env), "value");
        assert_eq!(Interpolator::interpolate("$MY_VAR", &env), "value");
    }

    #[test]
    fn test_numbers_in_variable_name() {
        let env = make_env(&[("VAR123", "value")]);
        assert_eq!(Interpolator::interpolate("${VAR123}", &env), "value");
        assert_eq!(Interpolator::interpolate("$VAR123", &env), "value");
    }
}
