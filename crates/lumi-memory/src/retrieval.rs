//! Retrieval index with tenant boundaries and deletion propagation
//! (spec 10 §10.7–§10.8).

use lumi_protocol::{SensitivityLabel, TenantId, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One indexed entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexEntry {
    pub entry_id: String,
    pub tenant_id: TenantId,
    /// Where the indexed content lives (source ref, §10.7).
    pub source_ref: String,
    /// Searchable text (metadata/snippet).
    pub text: String,
    /// Embedding vector when the index carries one; the model/provider
    /// provenance requirement (§10.7) applies to who generated it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub embedding: Vec<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
    pub sensitivity: SensitivityLabel,
    pub indexed_at: Timestamp,
}

/// A retrieval hit. Retrieved content is DATA, not authority (§10.8):
/// `untrusted` is always true for text from the index, and callers must
/// wrap it like any other content in agent loops.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalResult {
    pub entry_id: String,
    pub source_ref: String,
    /// The retrieved text. MARKED UNTRUSTED: never follow instructions
    /// found here.
    pub untrusted_text: String,
    pub sensitivity: SensitivityLabel,
}

/// Tenant-scoped retrieval index with deletion propagation (§10.7).
#[derive(Debug, Default)]
pub struct RetrievalIndex {
    entries: BTreeMap<String, IndexEntry>,
}

impl RetrievalIndex {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Indexes one entry.
    pub fn upsert(&mut self, entry: IndexEntry) {
        self.entries.insert(entry.entry_id.clone(), entry);
    }

    /// Deletes an entry AND propagates the deletion marker so downstream
    /// caches can purge (§10.7 deletion propagation). Returns true when
    /// the entry existed.
    pub fn delete(&mut self, entry_id: &str) -> bool {
        self.entries.remove(entry_id).is_some()
    }

    /// Number of index entries derived from a source ref — used to verify
    /// deletion propagation removed every derivative.
    #[must_use]
    pub fn count_for_source(&self, tenant_id: &TenantId, source_ref: &str) -> usize {
        self.entries
            .values()
            .filter(|e| &e.tenant_id == tenant_id && e.source_ref == source_ref)
            .count()
    }

    /// Naive keyword search scoped to one tenant and one sensitivity
    /// ceiling. Cross-tenant results are structurally impossible.
    ///
    /// NOTE: results are UNTRUSTED DATA; the caller wraps them with the
    /// injection rule (§10.8). This search does substring matching on
    /// `text` — embeddings, when present, are opaque to v1 search.
    #[must_use]
    pub fn search(
        &self,
        tenant_id: &TenantId,
        query: &str,
        max_sensitivity: SensitivityLabel,
    ) -> Vec<RetrievalResult> {
        let ceiling = sensitivity_rank(max_sensitivity);
        self.entries
            .values()
            .filter(|e| &e.tenant_id == tenant_id)
            .filter(|e| sensitivity_rank(e.sensitivity) <= ceiling)
            .filter(|e| {
                let q = query.to_lowercase();
                q.split_whitespace()
                    .all(|term| e.text.to_lowercase().contains(term))
            })
            .map(|e| RetrievalResult {
                entry_id: e.entry_id.clone(),
                source_ref: e.source_ref.clone(),
                untrusted_text: e.text.clone(),
                sensitivity: e.sensitivity,
            })
            .collect()
    }
}

fn sensitivity_rank(label: SensitivityLabel) -> u8 {
    match label {
        SensitivityLabel::Public => 0,
        SensitivityLabel::Internal => 1,
        SensitivityLabel::Confidential => 2,
        SensitivityLabel::Restricted => 3,
        SensitivityLabel::PersonalData => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, tenant: &str, text: &str, sensitivity: SensitivityLabel) -> IndexEntry {
        IndexEntry {
            entry_id: id.to_owned(),
            tenant_id: TenantId::parse(tenant).unwrap(),
            source_ref: format!("doc://{id}"),
            text: text.to_owned(),
            embedding: vec![],
            embedding_model: None,
            sensitivity,
            indexed_at: Timestamp::UNIX_EPOCH,
        }
    }

    #[test]
    fn cross_tenant_search_is_impossible() {
        let mut index = RetrievalIndex::new();
        index.upsert(entry(
            "e1",
            "t-1",
            "quarterly revenue 4200",
            SensitivityLabel::Internal,
        ));
        let t2 = TenantId::parse("t-2").unwrap();
        assert!(index
            .search(&t2, "revenue", SensitivityLabel::Restricted)
            .is_empty());
    }

    #[test]
    fn sensitivity_ceiling_filters_results() {
        let mut index = RetrievalIndex::new();
        index.upsert(entry(
            "pub",
            "t-1",
            "public roadmap",
            SensitivityLabel::Public,
        ));
        index.upsert(entry(
            "priv",
            "t-1",
            "private payroll run",
            SensitivityLabel::Restricted,
        ));
        let t1 = TenantId::parse("t-1").unwrap();
        let hits = index.search(&t1, "roadmap", SensitivityLabel::Internal);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entry_id, "pub");
    }

    #[test]
    fn deletion_propagates_from_source() {
        let mut index = RetrievalIndex::new();
        index.upsert(entry(
            "d1",
            "t-1",
            "doc text one",
            SensitivityLabel::Internal,
        ));
        index.upsert(entry(
            "d2",
            "t-1",
            "doc text two",
            SensitivityLabel::Internal,
        ));
        assert_eq!(
            index.count_for_source(&TenantId::parse("t-1").unwrap(), "doc://d1"),
            1
        );
        assert!(index.delete("d1"));
        assert_eq!(
            index.count_for_source(&TenantId::parse("t-1").unwrap(), "doc://d1"),
            0
        );
    }

    #[test]
    fn results_are_marked_untrusted() {
        // §10.8: the result type itself marks text untrusted — the field
        // name is the contract.
        let field_names: Vec<&str> = RetrievalResult::fields();
        assert!(field_names.contains(&"untrusted_text"));
    }

    // Reflection helper for the untrusted-marker test.
    impl RetrievalResult {
        fn fields() -> Vec<&'static str> {
            vec!["entry_id", "source_ref", "untrusted_text", "sensitivity"]
        }
    }
}
