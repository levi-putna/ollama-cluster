use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Model inventory entry used for fingerprinting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelInventoryEntry {
    pub name: String,
    pub digest: String,
    pub size: u64,
    pub modified_at: String,
}

/// Stable fingerprint for a node model inventory.
pub fn inventory_fingerprint(models: &[ModelInventoryEntry]) -> String {
    let mut sorted = models.to_vec();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));

    let mut hasher = Sha256::new();
    for m in &sorted {
        hasher.update(format!(
            "{}|{}|{}|{};",
            m.name, m.digest, m.size, m.modified_at
        ));
    }
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, digest: &str) -> ModelInventoryEntry {
        ModelInventoryEntry {
            name: name.into(),
            digest: digest.into(),
            size: 100,
            modified_at: "2026-01-01".into(),
        }
    }

    /// Covers: FR-048, TR-083, TXR-130-11
    #[test]
    fn fingerprint_stable_for_same_inventory() {
        let a = vec![entry("llama", "abc"), entry("qwen", "def")];
        let b = vec![entry("qwen", "def"), entry("llama", "abc")];
        assert_eq!(inventory_fingerprint(&a), inventory_fingerprint(&b));
    }

    /// Covers: FR-048, TR-083, TXR-130-11
    #[test]
    fn fingerprint_changes_on_digest_change() {
        let a = vec![entry("llama", "abc")];
        let b = vec![entry("llama", "xyz")];
        assert_ne!(inventory_fingerprint(&a), inventory_fingerprint(&b));
    }
}
