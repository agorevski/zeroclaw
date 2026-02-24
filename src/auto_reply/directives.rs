use super::traits::{DirectiveParser, ParsedDirective, ParsedMessage};

const SUPPORTED: &[&str] = &["@model", "@think", "@reason", "@verbose", "@spawn"];

/// Default directive parser that extracts `@name` and `@name(value)` patterns.
pub struct DefaultDirectiveParser;

impl DirectiveParser for DefaultDirectiveParser {
    fn parse(&self, text: &str) -> ParsedMessage {
        let mut directives = Vec::new();
        let mut slash_command = None;
        let mut slash_args = None;
        let mut clean_parts = Vec::new();

        for line in text.lines() {
            let trimmed = line.trim();

            // Check for slash command on first non-empty line if none found yet
            if slash_command.is_none() && trimmed.starts_with('/') {
                let rest = &trimmed[1..];
                if let Some(space_pos) = rest.find(' ') {
                    slash_command = Some(format!("/{}", &rest[..space_pos]));
                    let args = rest[space_pos..].trim();
                    if !args.is_empty() {
                        slash_args = Some(args.to_string());
                    }
                } else {
                    slash_command = Some(trimmed.to_string());
                }
                continue;
            }

            let processed = extract_directives(trimmed, &mut directives);
            if !processed.is_empty() {
                clean_parts.push(processed);
            }
        }

        ParsedMessage {
            clean_text: clean_parts.join("\n"),
            directives,
            slash_command,
            slash_args,
        }
    }

    fn supported_directives(&self) -> Vec<&str> {
        SUPPORTED.to_vec()
    }

    fn name(&self) -> &str {
        "default"
    }
}

/// Extract `@name` and `@name(value)` from a line, returning the cleaned text.
fn extract_directives(line: &str, directives: &mut Vec<ParsedDirective>) -> String {
    let mut clean = line.to_string();

    for &directive in SUPPORTED {
        let bare = &directive[1..]; // strip leading @
                                    // Match @name(value)
        let paren_prefix = format!("@{bare}(");
        while let Some(start) = clean.find(&paren_prefix) {
            if let Some(end) = clean[start..].find(')') {
                let value_start = start + paren_prefix.len();
                let value_end = start + end;
                let value = clean[value_start..value_end].to_string();
                directives.push(ParsedDirective {
                    name: bare.to_string(),
                    value: Some(value),
                });
                clean.replace_range(start..=(start + end), "");
            } else {
                break;
            }
        }

        // Match bare @name (must be at word boundary)
        let at_name = format!("@{bare}");
        while let Some(start) = clean.find(&at_name) {
            let after = start + at_name.len();
            // Ensure it's not part of a longer word or followed by '('
            if after < clean.len() {
                let next_char = clean.as_bytes()[after];
                if next_char.is_ascii_alphanumeric() || next_char == b'(' {
                    break;
                }
            }
            directives.push(ParsedDirective {
                name: bare.to_string(),
                value: None,
            });
            clean.replace_range(start..after, "");
        }
    }

    // Collapse runs of whitespace left after directive removal
    clean.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bare_directive() {
        let parser = DefaultDirectiveParser;
        let result = parser.parse("hello @model world");
        assert_eq!(result.directives.len(), 1);
        assert_eq!(result.directives[0].name, "model");
        assert!(result.directives[0].value.is_none());
        assert_eq!(result.clean_text, "hello world");
    }

    #[test]
    fn parse_directive_with_value() {
        let parser = DefaultDirectiveParser;
        let result = parser.parse("use @model(gpt-4) please");
        assert_eq!(result.directives.len(), 1);
        assert_eq!(result.directives[0].name, "model");
        assert_eq!(result.directives[0].value.as_deref(), Some("gpt-4"));
        assert_eq!(result.clean_text, "use please");
    }

    #[test]
    fn parse_slash_command() {
        let parser = DefaultDirectiveParser;
        let result = parser.parse("/help some args");
        assert_eq!(result.slash_command.as_deref(), Some("/help"));
        assert_eq!(result.slash_args.as_deref(), Some("some args"));
        assert!(result.clean_text.is_empty());
    }

    #[test]
    fn parse_slash_command_no_args() {
        let parser = DefaultDirectiveParser;
        let result = parser.parse("/compact");
        assert_eq!(result.slash_command.as_deref(), Some("/compact"));
        assert!(result.slash_args.is_none());
    }

    #[test]
    fn parse_no_directives() {
        let parser = DefaultDirectiveParser;
        let result = parser.parse("just a plain message");
        assert!(result.directives.is_empty());
        assert!(result.slash_command.is_none());
        assert_eq!(result.clean_text, "just a plain message");
    }

    #[test]
    fn parse_multiple_directives() {
        let parser = DefaultDirectiveParser;
        let result = parser.parse("@verbose @think do the thing");
        assert_eq!(result.directives.len(), 2);
        let names: Vec<&str> = result.directives.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"verbose"));
        assert!(names.contains(&"think"));
    }

    #[test]
    fn supported_directives_list() {
        let parser = DefaultDirectiveParser;
        let supported = parser.supported_directives();
        assert!(supported.contains(&"@model"));
        assert!(supported.contains(&"@spawn"));
        assert_eq!(supported.len(), 5);
    }
}
