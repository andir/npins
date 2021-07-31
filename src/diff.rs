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
}
