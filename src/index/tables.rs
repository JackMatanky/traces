//! Defines redb table schemas and rebuild deletion policies for the index
//! store.
//!
//! [`IndexStore`] owns connections and row payloads; this module owns the
//! static schema: eight `TableSpec` entries pairing each table's definition
//! with its rebuild wipe policy.
//!
//! [`IndexStore`]: super::store::IndexStore

use redb::{
    MultimapTableDefinition, ReadTransaction, TableDefinition, WriteTransaction,
};

use super::{error::StoreResult, store::IndexStore};

/// Defines the file metadata table.
///
/// Key: project-relative path as UTF-8 bytes
/// Value: serialized [`crate::FileMeta`]
pub(super) const FILES: TableDefinition<'static, &'static [u8], &'static [u8]> =
    TableDefinition::new("files");

/// Defines the parsed note metadata table.
///
/// Key: project-relative path as UTF-8 bytes
/// Value: serialized [`crate::Note`]
pub(super) const NOTES: TableDefinition<'static, &'static [u8], &'static [u8]> =
    TableDefinition::new("notes");

/// Defines the inbound link multimap table.
///
/// Key: target note path as UTF-8 bytes
/// Value: one source note path per entry
pub(super) const LINKS: MultimapTableDefinition<
    'static,
    &'static [u8],
    &'static [u8],
> = MultimapTableDefinition::new("links");

/// Defines the path lookup by tag multimap table.
pub(super) const PATHS_BY_TAG: MultimapTableDefinition<
    'static,
    &'static [u8],
    &'static [u8],
> = MultimapTableDefinition::new("paths_by_tag");

/// Defines the path lookup by file class multimap table.
pub(super) const PATHS_BY_FILE_CLASS: MultimapTableDefinition<
    'static,
    &'static [u8],
    &'static [u8],
> = MultimapTableDefinition::new("paths_by_file_class");

/// Defines the reverse tag index mapping paths to current normalized tags.
///
/// Lets incremental upserts and deletes touch O(path tag count) entries instead
/// of scanning [`PATHS_BY_TAG`].
pub(super) const TAGS_BY_PATH: MultimapTableDefinition<
    'static,
    &'static [u8],
    &'static [u8],
> = MultimapTableDefinition::new("tags_by_path");

/// Defines the reverse file-class index mapping paths to current normalized
/// classes.
pub(super) const FILE_CLASSES_BY_PATH: MultimapTableDefinition<
    'static,
    &'static [u8],
    &'static [u8],
> = MultimapTableDefinition::new("classes_by_path");

/// Defines the store's single metadata table.
///
/// Key `version`: the cache format version identifying persisted row semantics.
/// Key `fingerprint`: the raw digests of the index-relevant config settings
/// that produced the rows, as written by
/// [`super::fingerprint::IndexFingerprint`].
pub(super) const META: TableDefinition<'static, &'static [u8], &'static [u8]> =
    TableDefinition::new("meta");

/// Represents a schema table: to-one row or to-many multimap.
enum TableDef {
    ToOne(TableDefinition<'static, &'static [u8], &'static [u8]>),
    ToMany(MultimapTableDefinition<'static, &'static [u8], &'static [u8]>),
}

/// Specifies how a table's rows are treated during a rebuild wipe.
#[derive(Copy, Clone, Eq, PartialEq)]
enum DeletePolicy {
    /// A delete error fails the rebuild.
    Required,
    /// Only a missing table is tolerated; other errors propagate.
    BestEffort,
}

/// Pairs a table definition with its rebuild deletion policy.
pub(super) struct TableSpec {
    def: TableDef,
    policy: DeletePolicy,
}

/// Lists every table in the schema in declaration order.
pub(super) const TABLES: [TableSpec; 8] = [
    TableSpec {
        def: TableDef::ToOne(FILES),
        policy: DeletePolicy::Required,
    },
    TableSpec {
        def: TableDef::ToOne(NOTES),
        policy: DeletePolicy::Required,
    },
    TableSpec {
        def: TableDef::ToMany(LINKS),
        policy: DeletePolicy::Required,
    },
    TableSpec {
        def: TableDef::ToMany(PATHS_BY_TAG),
        policy: DeletePolicy::BestEffort,
    },
    TableSpec {
        def: TableDef::ToMany(PATHS_BY_FILE_CLASS),
        policy: DeletePolicy::BestEffort,
    },
    TableSpec {
        def: TableDef::ToMany(TAGS_BY_PATH),
        policy: DeletePolicy::BestEffort,
    },
    TableSpec {
        def: TableDef::ToMany(FILE_CLASSES_BY_PATH),
        policy: DeletePolicy::BestEffort,
    },
    TableSpec {
        def: TableDef::ToOne(META),
        policy: DeletePolicy::BestEffort,
    },
];

impl TableDef {
    /// Probes the definition against `txn` without reading rows.
    fn probe(&self, txn: &ReadTransaction) -> Result<(), redb::TableError> {
        match self {
            Self::ToOne(def) => txn.open_table(*def).map(|_| ()),
            Self::ToMany(def) => txn.open_multimap_table(*def).map(|_| ()),
        }
    }
}

impl TableSpec {
    /// Probes this table's definition against `txn` without reading rows.
    ///
    /// # Errors
    ///
    /// - [`redb::TableError`] if the table definition cannot be opened or does
    ///   not match.
    pub(super) fn probe(
        &self,
        txn: &ReadTransaction,
    ) -> Result<(), redb::TableError> {
        self.def.probe(txn)
    }

    /// Deletes this table's contents per its [`DeletePolicy`].
    ///
    /// `Required` propagates every storage error; `BestEffort` tolerates only a
    /// missing table (the fresh-database case) and propagates the rest.
    ///
    /// # Errors
    ///
    /// - [`StoreError::Redb`] if deleting the table fails in redb and policy
    ///   does not permit ignoring it.
    ///
    /// [`StoreError::Redb`]: super::error::StoreError::Redb
    pub(super) fn delete(
        &self,
        store: &IndexStore,
        txn: &WriteTransaction,
    ) -> StoreResult<()> {
        let run = || match &self.def {
            TableDef::ToOne(def) => txn.delete_table(*def).map(|_| ()),
            TableDef::ToMany(def) => {
                txn.delete_multimap_table(*def).map(|_| ())
            }
        };
        match run() {
            Ok(()) => Ok(()),
            Err(redb::TableError::TableDoesNotExist(_))
                if self.policy == DeletePolicy::BestEffort =>
            {
                Ok(())
            }
            Err(source) => Err(store.wrap_redb_error(source)),
        }
    }
}
