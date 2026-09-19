//! Schema migration chains (spec 20 §20.7).
//!
//! Migrations are versioned, deterministic, tested, and reversible where
//! practical. Each step upgrades one version to the next; a chain
//! composes steps. A destructive migration requires an explicit
//! irreversible flag (§20.13).

use std::collections::BTreeMap;

/// One migration step: `from_version` → `to_version`.
pub struct Migration {
    pub from_version: u32,
    pub to_version: u32,
    /// True when the migration cannot be undone (§20.13).
    pub irreversible: bool,
    /// Deterministic transform function.
    transform: Box<dyn Fn(serde_json::Value) -> Result<serde_json::Value, String> + Send + Sync>,
}

impl std::fmt::Debug for Migration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Migration")
            .field("from", &self.from_version)
            .field("to", &self.to_version)
            .field("irreversible", &self.irreversible)
            .finish()
    }
}

/// Migration chain errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationError {
    /// No migration step registered for this source version.
    NoMigration { from: u32 },
    /// A required destructive migration was not acknowledged.
    DestructiveRequiresAck { from: u32, to: u32 },
    /// The transform function failed.
    Transform { from: u32, detail: String },
    /// The migration output still claims the old version.
    TransformDidNotBumpVersion { from: u32 },
}

impl std::fmt::Display for MigrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoMigration { from } => write!(f, "no migration from schema v{from}"),
            Self::DestructiveRequiresAck { from, to } => {
                write!(
                    f,
                    "destructive migration v{from}→v{to} requires explicit acknowledgment"
                )
            }
            Self::Transform { from, detail } => {
                write!(f, "migration from v{from} failed: {detail}")
            }
            Self::TransformDidNotBumpVersion { from } => {
                write!(f, "migration from v{from} did not bump schema_version")
            }
        }
    }
}

impl std::error::Error for MigrationError {}

/// A migration chain, keyed by source version.
#[derive(Default)]
pub struct MigrationChain {
    steps: BTreeMap<u32, Migration>,
}

impl MigrationChain {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a migration step.
    pub fn push(&mut self, migration: Migration) {
        self.steps.insert(migration.from_version, migration);
    }

    /// Runs migrations sequentially until the target version is reached.
    /// Every intermediate state is a valid serde_json::Value (the caller
    /// snapshots before starting, §20.7: backed up).
    ///
    /// `acknowledge_destructive` must be true to run an irreversible
    /// migration (§20.13).
    ///
    /// # Errors
    /// [`MigrationError`] for missing steps, unacknowledged destructive
    /// transforms, or transform failures.
    pub fn migrate(
        &self,
        data: &mut serde_json::Value,
        current_version: u32,
        target_version: u32,
        acknowledge_destructive: bool,
    ) -> Result<(), MigrationError> {
        let mut version = current_version;
        while version < target_version {
            let step = self
                .steps
                .get(&version)
                .ok_or(MigrationError::NoMigration { from: version })?;
            if step.irreversible && !acknowledge_destructive {
                return Err(MigrationError::DestructiveRequiresAck {
                    from: step.from_version,
                    to: step.to_version,
                });
            }
            let new_data =
                (step.transform)(data.clone()).map_err(|e| MigrationError::Transform {
                    from: version,
                    detail: e,
                })?;

            // The migration MUST bump the schema_version field.
            let bumped = new_data
                .get("schema_version")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32);
            match bumped {
                Some(v) if v == step.to_version => {}
                _ => {
                    return Err(MigrationError::TransformDidNotBumpVersion { from: version });
                }
            }

            *data = new_data;
            version = step.to_version;
        }
        if version > target_version {
            // Over-migrated: a bug in the chain definition.
            return Err(MigrationError::Transform {
                from: version,
                detail: format!("overshot target {target_version}"),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn chain_runs_steps_sequentially() {
        let mut chain = MigrationChain::new();
        chain.push(Migration {
            from_version: 1,
            to_version: 2,
            irreversible: false,
            transform: Box::new(|mut v| {
                v["schema_version"] = json!(2);
                v["renamed_field"] = v["old_field"].clone();
                v.as_object_mut().unwrap().remove("old_field");
                Ok(v)
            }),
        });
        chain.push(Migration {
            from_version: 2,
            to_version: 3,
            irreversible: false,
            transform: Box::new(|mut v| {
                v["schema_version"] = json!(3);
                v["new_field"] = json!("default");
                Ok(v)
            }),
        });

        let mut data = json!({
            "schema_version": 1,
            "old_field": "value",
        });
        chain.migrate(&mut data, 1, 3, false).unwrap();
        assert_eq!(data["schema_version"], 3);
        assert_eq!(data["renamed_field"], "value");
        assert_eq!(data["new_field"], "default");
    }

    #[test]
    fn destructive_migration_requires_acknowledgment() {
        let mut chain = MigrationChain::new();
        chain.push(Migration {
            from_version: 1,
            to_version: 2,
            irreversible: true,
            transform: Box::new(|mut v| {
                v["schema_version"] = json!(2);
                Ok(v)
            }),
        });
        let mut data = json!({"schema_version": 1});
        // Without ack: refused.
        assert!(matches!(
            chain.migrate(&mut data, 1, 2, false),
            Err(MigrationError::DestructiveRequiresAck { .. })
        ));
        // With ack: runs.
        chain.migrate(&mut data, 1, 2, true).unwrap();
        assert_eq!(data["schema_version"], 2);
    }

    #[test]
    fn missing_step_fails_explicitly() {
        let chain = MigrationChain::new();
        let mut data = json!({"schema_version": 1});
        assert!(matches!(
            chain.migrate(&mut data, 1, 2, false),
            Err(MigrationError::NoMigration { from: 1 })
        ));
    }

    #[test]
    fn transform_not_bumping_version_fails() {
        let mut chain = MigrationChain::new();
        chain.push(Migration {
            from_version: 1,
            to_version: 2,
            irreversible: false,
            transform: Box::new(|v| {
                // Deliberately NOT bumping schema_version: this tests the
                // version-bump check.
                Ok(v)
            }), // didn't bump (intentionally, to test the check)
        });
        let mut data = json!({"schema_version": 1});
        assert!(matches!(
            chain.migrate(&mut data, 1, 2, false),
            Err(MigrationError::TransformDidNotBumpVersion { from: 1 })
        ));
    }
}
