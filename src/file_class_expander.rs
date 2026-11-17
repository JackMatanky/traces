//! Implements [`FileClassExpander`] for [`SchemaService`].
//!
//! This module is the only place in the crate that references both `schema` and
//! `query` for File Class expansion. Neither module depends on the other. This
//! module connects them.
//!
//! [`FileClassExpander`]: crate::query::FileClassExpander
//! [`SchemaService`]: crate::schema::SchemaService

use indexmap::IndexSet;

use crate::{
    query::{ClassExpansionMode, FileClassExpander},
    schema::SchemaService,
};

impl FileClassExpander for SchemaService {
    fn expand(&self, classes: &[String], mode: &mut ClassExpansionMode) {
        let mut expanded: IndexSet<String> = classes.iter().cloned().collect();
        match mode {
            ClassExpansionMode::Exact(_) => {
                self.warn_unknown_classes(classes);
            }
            ClassExpansionMode::Children(_) => {
                self.warn_unknown_classes(classes);
                for class in classes {
                    expanded.extend(
                        self.children_names_of(class).map(str::to_owned),
                    );
                }
            }
            ClassExpansionMode::Descendants(_) => {
                // `matches` logs unknown classes internally, so this branch
                // skips `warn_unknown_classes`.
                expanded = self.matches(classes);
            }
        }
        mode.set_classes(expanded.into_iter().collect());
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, fs, path::Path};

    use pretty_assertions::assert_eq;

    use super::*;

    fn write_schema(dir: &Path, name: &str, toml: &str) {
        fs::write(dir.join(format!("{name}.toml")), toml)
            .expect("write schema fixture");
    }

    /// Builds a `thing` → `book` → `sci_fi` → `space_opera` extension chain.
    fn service(temp: &tempfile::TempDir) -> SchemaService {
        write_schema(temp.path(), "thing", "");
        write_schema(temp.path(), "book", r#"extends = ["thing"]"#);
        write_schema(temp.path(), "sci_fi", r#"extends = ["book"]"#);
        write_schema(temp.path(), "space_opera", r#"extends = ["sci_fi"]"#);
        SchemaService::new(temp.path()).expect("registry loads")
    }

    fn set(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn expansion_modes_are_incremental() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let service = service(&temp);
        let names = vec!["thing".to_owned()];
        let mut exact = ClassExpansionMode::Exact(BTreeSet::new());
        let mut children = ClassExpansionMode::Children(BTreeSet::new());
        let mut descendants = ClassExpansionMode::Descendants(BTreeSet::new());

        service.expand(&names, &mut exact);
        service.expand(&names, &mut children);
        service.expand(&names, &mut descendants);

        assert_eq!(exact.classes(), &set(&["thing"]));
        assert_eq!(children.classes(), &set(&["book", "thing"]));
        assert_eq!(
            descendants.classes(),
            &set(&["book", "sci_fi", "space_opera", "thing"])
        );
    }
}
