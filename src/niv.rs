//! Convert+Import Niv files

use crate::{git, Pin};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

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

    fn try_from(niv: NivPin) -> Result<Self> {
        Ok(match niv.owner {
            None => git::GitPin::git(niv.repo.parse()?, niv.branch).into(),
            Some(owner) => git::GitPin::github(&owner, &niv.repo, niv.branch).into(),
        })
    }
}
