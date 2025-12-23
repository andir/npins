//! Helper tools for generating differential output

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Change {
    Added(String),
    Removed(String),
    Changed(String, String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffEntry(String, Change);

impl std::fmt::Display for DiffEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let DiffEntry(property, change) = self;
        match change {
            Change::Removed(text) | Change::Changed(text, _) => {
                writeln!(f, "-    {}: {}", property, text)?;
            },
            _ => (),
        }
        match change {
            Change::Added(text) | Change::Changed(_, text) => {
                writeln!(f, "+    {}: {}", property, text)?;
            },
            _ => (),
        }
        Ok(())
    }
}

/// Diff two sets of properties
///
/// The code works best when both sides have the same keys.
pub fn diff(old: &[(String, String)], new: &[(String, String)]) -> Vec<DiffEntry> {
    let mut changes = Vec::with_capacity(5);
    let mut old_keys = std::collections::HashMap::new();

    old_keys.extend(old.iter().cloned());

    /* Iterate over the new values and check if the old ones exist */
    for (new_key, new_value) in new {
        match old_keys.remove(new_key) {
            Some(old_value) => {
                if &old_value != new_value {
                    changes.push(DiffEntry(
                        new_key.clone(),
                        Change::Changed(old_value, new_value.clone()),
                    ));
                }
            },
            None => {
                changes.push(DiffEntry(new_key.clone(), Change::Added(new_value.clone())));
            },
        }
    }

    /* All remaining keys that weren't matched were removed */
    for (key, value) in old_keys {
        changes.push(DiffEntry(key.clone(), Change::Removed(value)));
    }

    changes
}

pub trait Diff {
    /// List the key-value properties for this struct. Order matters
    fn properties(&self) -> Vec<(String, String)>;
}

impl<T: Diff> Diff for Option<T> {
    fn properties(&self) -> Vec<(String, String)> {
        self.as_ref().map(Diff::properties).unwrap_or_default()
    }
}

pub trait OptionExt<T> {
    /// Like [`Option::insert`] but returns the diff between both values instead of a reference to the inserted value.
    ///
    /// If `self` is `None`, then the diff will always be empty.
    fn insert_diffed(&mut self, value: T) -> Vec<DiffEntry>;
}

impl<T> OptionExt<T> for Option<T>
where
    T: Diff,
{
    fn insert_diffed(&mut self, value: T) -> Vec<DiffEntry> {
        let diff = match self {
            Some(this) => diff(&this.properties(), &value.properties()),
            None => value
                .properties()
                .into_iter()
                .map(|(key, val)| DiffEntry(key, Change::Added(val)))
                .collect(),
        };
        *self = Some(value);
        diff
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct TestObject;

    impl Diff for TestObject {
        fn properties(&self) -> Vec<(String, String)> {
            vec![("foo".into(), "bar".into())]
        }
    }

    #[test]
    fn test_option_diff() {
        assert_eq!(Some(TestObject).properties(), TestObject.properties());
        assert_eq!(None::<TestObject>.properties(), vec![]);

        assert_eq!(
            None.insert_diffed(TestObject),
            vec![DiffEntry("foo".into(), Change::Added("bar".into()))],
        );
        assert_eq!(Some(TestObject).insert_diffed(TestObject), vec![],);
    }

    #[test]
    fn test_diff() {
        /* Some test strings */
        let foo = || "foo".to_string();
        let bar = || "bar".to_string();
        let baz = || "baz".to_string();
        let empty = || "".to_string();

        assert_eq!(diff(&[], &[]), vec![],);
        assert_eq!(
            diff(&[(foo(), empty())], &[(bar(), empty())],),
            vec![
                DiffEntry(bar(), Change::Added(empty())),
                DiffEntry(foo(), Change::Removed(empty())),
            ],
        );
        assert_eq!(
            diff(&[(baz(), foo())], &[(baz(), bar())],),
            vec![DiffEntry(baz(), Change::Changed(foo(), bar())),],
        );
    }
}
