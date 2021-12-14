#[inline]
pub fn d(list: &[Option<Difference>]) -> Vec<Difference> {
    list.iter().cloned().filter_map(|x| x).collect()
}

#[derive(Clone)]
pub struct Difference {
    field: String,
    old: String,
    new: String,
}

impl Difference {
    pub fn new<T>(field: impl AsRef<str>, a: &T, b: &T) -> Option<Self>
    where
        T: std::fmt::Debug + std::cmp::PartialEq,
    {
        if a != b {
            let a = format!("{:?}", a);
            let b = format!("{:?}", b);
            Some(Difference {
                field: field.as_ref().to_owned(),
                old: a,
                new: b,
            })
        } else {
            None
        }
    }
}

impl std::fmt::Display for Difference {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "{}:\n", self.field)?;
        write!(fmt, "\t- {}\n", self.old)?;
        write!(fmt, "\t+ {}\n", self.new)?;

        Ok(())
    }
}

pub trait Diff {
    fn diff(&self, other: &Self) -> Vec<Difference>;

    /// Set `self` to `other` and return the [`diff`](Self::diff).
    fn set(&mut self, other: Self) -> Vec<Difference>
    where
        Self: Sized,
    {
        let diff = self.diff(&other);
        *self = other;
        diff
    }
}

pub trait OptionExt<T> {
    fn insert_diffed(&mut self, value: T) -> Vec<Difference>;
}

impl<T> OptionExt<T> for Option<T>
where
    T: Diff,
{
    /// Like [`Option::insert`] but returns the diff between both values instead of a reference to the inserted value.
    ///
    /// If `self` is `None`, then the diff will always be empty.
    fn insert_diffed(&mut self, value: T) -> Vec<Difference> {
        match self {
            Some(this) => this.set(value),
            None => {
                *self = Some(value);
                Vec::new()
            },
        }
    }
}
