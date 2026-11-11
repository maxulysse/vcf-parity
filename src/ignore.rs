/// `--ignore` patterns, split into `/`-separated segments once.
pub struct Patterns(Vec<Vec<String>>);

impl Patterns {
    pub fn new(patterns: &[String]) -> Self {
        Patterns(
            patterns
                .iter()
                .map(|p| p.split('/').map(String::from).collect())
                .collect(),
        )
    }

    /// Whether `field` is a pattern or below one, `*` matching any one segment.
    pub fn matches(&self, field: &str) -> bool {
        self.0.iter().any(|pattern| {
            let mut segments = field.split('/');
            pattern
                .iter()
                .all(|p| segments.next().is_some_and(|s| p == "*" || p == s))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_segments_wildcards_and_prefixes() {
        let patterns = Patterns::new(&["ID".into(), "header/*/*/Description".into()]);
        for field in ["ID", "header/INFO/DP/Description"] {
            assert!(patterns.matches(field), "{field}");
        }
        for field in ["IDX", "INFO/ID", "header/INFO/DP", "header/INFO/DP/Type"] {
            assert!(!patterns.matches(field), "{field}");
        }
        assert!(Patterns::new(&["header".into()]).matches("header/contig/chr1/length"));
    }
}
