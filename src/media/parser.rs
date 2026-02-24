use super::traits::{MediaParser, MediaToken};

/// Default parser that extracts `MEDIA: <path_or_url>` tokens from text.
pub struct DefaultMediaParser;

impl MediaParser for DefaultMediaParser {
    fn parse_tokens(&self, text: &str) -> Vec<MediaToken> {
        let mut tokens = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("MEDIA:") {
                let source = value.trim().to_string();
                if source.is_empty() {
                    continue;
                }
                let is_url = source.starts_with("http://") || source.starts_with("https://");
                tokens.push(MediaToken { source, is_url });
            }
        }
        tokens
    }

    fn name(&self) -> &str {
        "default"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_url_token() {
        let parser = DefaultMediaParser;
        let tokens = parser.parse_tokens("MEDIA: https://example.com/image.png");
        assert_eq!(tokens.len(), 1);
        assert!(tokens[0].is_url);
        assert_eq!(tokens[0].source, "https://example.com/image.png");
    }

    #[test]
    fn parses_path_token() {
        let parser = DefaultMediaParser;
        let tokens = parser.parse_tokens("MEDIA: /tmp/file.txt");
        assert_eq!(tokens.len(), 1);
        assert!(!tokens[0].is_url);
    }

    #[test]
    fn ignores_non_media_lines() {
        let parser = DefaultMediaParser;
        let tokens = parser.parse_tokens("Hello world\nMEDIA: /a.bin\nfoo");
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn skips_empty_media_value() {
        let parser = DefaultMediaParser;
        let tokens = parser.parse_tokens("MEDIA:   ");
        assert!(tokens.is_empty());
    }
}
