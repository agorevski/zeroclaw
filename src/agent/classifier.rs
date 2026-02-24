/// Classify a user message — currently a no-op after query classification
/// config was removed.
///
/// Always returns `None`.
pub fn classify(_message: &str) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_always_returns_none() {
        assert_eq!(classify("hello"), None);
        assert_eq!(classify("write some code"), None);
    }
}
