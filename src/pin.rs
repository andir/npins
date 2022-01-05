use crate::*;

/// Create the `Pin` type
///
/// We need a type to unify over all possible way to pin a dependency. Normally, this would be done with a trait
/// and trait objects. However, designing such a trait to be object-safe turns out to be highly non-trivial.

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Pin {
    GitHub {
        #[serde(flatten)]
        input: github::PinInput,
        #[serde(flatten)]
        output: Option<github::PinOutput>,
    },

    GitHubRelease {
        #[serde(flatten)]
        input: github::ReleasePinInput,
        #[serde(flatten)]
        output: Option<github::ReleasePinOutput>,
    },

    Git {
        #[serde(flatten)]
        input: git::PinInput,
        #[serde(flatten)]
        output: Option<git::PinOutput>,
    },

    PyPi {
        #[serde(flatten)]
        input: pypi::PinInput,
        #[serde(flatten)]
        output: Option<pypi::PinOutput>,
    },
}

impl Pin {
    pub fn github(input: github::PinInput) -> Self {
        Self::GitHub {
            input,
            output: None,
        }
    }

    pub fn github_release(input: github::ReleasePinInput) -> Self {
        Self::GitHubRelease {
            input,
            output: None,
        }
    }

    pub fn git(input: git::PinInput) -> Self {
        Self::Git {
            input,
            output: None,
        }
    }

    pub fn pypi(input: pypi::PinInput) -> Self {
        Self::PyPi {
            input,
            output: None,
        }
    }

    pub async fn update(&mut self) -> Result<Vec<diff::Difference>> {
        /* Use very explicit syntax to force the correct types and get good compile errors */
        Ok(match self {
            Self::GitHub { input, output } => {
                let new_output: github::PinOutput =
                    <github::PinInput as Updatable>::update(input).await?;
                output.insert_diffed(new_output)
            },

            Self::GitHubRelease { input, output } => {
                let new_output: github::ReleasePinOutput =
                    <github::ReleasePinInput as Updatable>::update(input).await?;
                output.insert_diffed(new_output)
            },

            Self::Git { input, output } => {
                let new_output: git::PinOutput =
                    <git::PinInput as Updatable>::update(input).await?;
                output.insert_diffed(new_output)
            },

            Self::PyPi { input, output } => {
                let new_output: pypi::PinOutput =
                    <pypi::PinInput as Updatable>::update(input).await?;
                output.insert_diffed(new_output)
            },
        })
    }
}

impl std::fmt::Display for Pin {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::GitHub { input, output } => write!(
                fmt,
                "GitHub:{}/{}@{} ({})",
                input.repository,
                input.owner,
                output.as_ref().map_or("", |o| &o.revision),
                input.branch
            ),

            Self::GitHubRelease { input, output } => write!(
                fmt,
                "GitHubRelease:{}/{}@{}",
                input.repository,
                input.owner,
                output.as_ref().map_or("", |o| &o.release_name)
            ),

            Self::Git { input, output } => write!(
                fmt,
                "Git:{}@{} ({})",
                input.repository_url,
                output.as_ref().map_or("", |o| &o.revision),
                input.branch
            ),

            Self::PyPi { input, output } => write!(
                fmt,
                "PyPi: {}@{}",
                input.name,
                output.as_ref().map_or("", |o| &o.version)
            ),
        }
    }
}
