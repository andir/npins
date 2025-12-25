//! Convert+Import Niv files

use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

use crate::{Pin, git};

/// Pin entry from Niv's sources.json
///
/// We only take the minimum information required to get things working. This does not include
/// the actual hashes, so an update must be performed afterwards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NivPin {
    branch: String,
    /// The GitHub repository owner. If not present, then `repo` contains the full git URL.
    owner: Option<String>,
    /// Might be a git URL. In that case, `owner` won't be present.
    repo: String,
}

impl TryFrom<NivPin> for Pin {
    type Error = anyhow::Error;

    fn try_from(niv: NivPin) -> anyhow::Result<Self> {
        Ok(match niv.owner {
            None => {
                git::GitPin::new(git::Repository::git(niv.repo.parse()?), niv.branch, false).into()
            },
            Some(owner) => git::GitPin::new(
                git::Repository::github(&owner, &niv.repo),
                niv.branch,
                false,
            )
            .into(),
        })
    }
}
