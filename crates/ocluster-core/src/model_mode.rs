use serde::{Deserialize, Serialize};

/// How effective model sets are calculated per node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModelMode {
    #[default]
    Discover,
    Allow,
    Static,
}

/// Compute effective models for a node based on discovery mode.
pub fn effective_models(
    mode: ModelMode,
    discovered: &[String],
    configured: &[String],
) -> Vec<String> {
    match mode {
        ModelMode::Discover => discovered.to_vec(),
        ModelMode::Allow => {
            let configured_set: std::collections::HashSet<_> = configured.iter().collect();
            discovered
                .iter()
                .filter(|m| configured_set.contains(m))
                .cloned()
                .collect()
        }
        ModelMode::Static => configured.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Covers: FR-041, TR-085, TXR-130-03
    #[test]
    fn discover_mode_uses_all_discovered() {
        let discovered = vec!["a".into(), "b".into()];
        let configured = vec!["b".into()];
        let effective = effective_models(ModelMode::Discover, &discovered, &configured);
        assert_eq!(effective, discovered);
    }

    /// Covers: FR-041, TR-085, TXR-130-04
    #[test]
    fn allow_mode_intersects() {
        let discovered = vec!["a".into(), "b".into(), "c".into()];
        let configured = vec!["b".into(), "d".into()];
        let effective = effective_models(ModelMode::Allow, &discovered, &configured);
        assert_eq!(effective, vec!["b".to_string()]);
    }

    /// Covers: FR-041, TR-085, TXR-130-05
    #[test]
    fn static_mode_uses_configured_only() {
        let discovered = vec!["a".into()];
        let configured = vec!["x".into(), "y".into()];
        let effective = effective_models(ModelMode::Static, &discovered, &configured);
        assert_eq!(effective, configured);
    }
}
