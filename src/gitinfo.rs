use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug, Display, Formatter};

/// Information about a git tag that is an ancestor of the current commit.
///
/// This struct contains the tag name and the number of commits between
/// the tagged commit and the current HEAD.
///
/// # Example
///
/// ```
/// use git2version::TagInfo;
///
/// let tag_info = TagInfo {
///     tag: "v1.2.3",
///     commits_since_tag: 5,
/// };
/// assert_eq!(tag_info.tag, "v1.2.3");
/// assert_eq!(tag_info.commits_since_tag, 5);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagInfo<'a> {
    /// The name of the tag (e.g., `"v1.2.3"`, `"release-1.0"`).
    pub tag: &'a str,

    /// The number of commits between the tagged commit and the current HEAD.
    /// This is `0` if HEAD is the tagged commit itself.
    pub commits_since_tag: u32,
}

/// Git version information extracted from a repository.
///
/// This struct contains information about the current commit, including
/// any ancestor tags, the commit ID, and whether the working directory
/// has uncommitted changes.
///
/// # Display Format
///
/// When converted to a string via [`Display`] or [`Debug`], `GitInfo` produces
/// output in the format: `{tag}+{commits}.g{commit_id}[.modified]`
///
/// Where:
/// - `{tag}` is the tag name, or `"unknown"` if no tag exists
/// - `{commits}` is the number of commits since the tag (omitted if no tag)
/// - `g{commit_id}` is the shortened commit hash prefixed with `'g'` (for "git")
/// - `.modified` is appended if the working directory has uncommitted changes
///
/// # Examples
///
/// ```
/// use git2version::{GitInfo, TagInfo};
///
/// // Version on a tag, clean working directory
/// let on_tag = GitInfo {
///     tag_info: Some(TagInfo {
///         tag: "v1.2.3",
///         commits_since_tag: 0,
///     }),
///     commit_id: "abcdef1234",
///     modified: false,
/// };
/// assert_eq!(format!("{}", on_tag), "v1.2.3+0.gabcdef1234");
///
/// // Version after a tag with modifications
/// let after_tag_modified = GitInfo {
///     tag_info: Some(TagInfo {
///         tag: "v1.2.3",
///         commits_since_tag: 5,
///     }),
///     commit_id: "abcdef1234",
///     modified: true,
/// };
/// assert_eq!(format!("{}", after_tag_modified), "v1.2.3+5.gabcdef1234.modified");
///
/// // No ancestor tag
/// let no_tag = GitInfo {
///     tag_info: None,
///     commit_id: "abcdef1234",
///     modified: false,
/// };
/// assert_eq!(format!("{}", no_tag), "unknown.gabcdef1234");
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct GitInfo<'a, 'b> {
    /// Information on the tag that is the closest ancestor tag to the current commit.
    ///
    /// This is `None` if the repository doesn't have any tags or is a shallow clone
    /// where tags aren't available.
    pub tag_info: Option<TagInfo<'a>>,

    /// The shortened ID of the current HEAD commit.
    ///
    /// This is a 10-character prefix of the full commit hash, as determined by
    /// [`COMMIT_ID_SHORT_HASH_LENGTH`](crate::COMMIT_ID_SHORT_HASH_LENGTH).
    pub commit_id: &'b str,

    /// Whether the working directory has uncommitted changes.
    ///
    /// This is `true` if there are staged or unstaged changes to tracked files.
    /// Untracked files are not considered modifications.
    pub modified: bool,
}

impl<'a, 'b> Debug for GitInfo<'a, 'b> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl<'a, 'b> Display for GitInfo<'a, 'b> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(tag) = self.tag_info {
            write!(f, "{}+{}", tag.tag, tag.commits_since_tag)?;
        } else {
            write!(f, "unknown")?;
        }
        write!(f, ".g{}", self.commit_id)?;
        if self.modified {
            write!(f, ".modified")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod display {
        use super::*;

        #[test]
        fn notag_notmodified() {
            let version = GitInfo {
                tag_info: None,
                commit_id: "abcdef",
                modified: false,
            };
            assert_eq!("unknown.gabcdef", format!("{}", version));
            assert_eq!("unknown.gabcdef", format!("{:?}", version));
        }

        #[test]
        fn notag_modified() {
            let version = GitInfo {
                tag_info: None,
                commit_id: "abcdef",
                modified: true,
            };
            assert_eq!("unknown.gabcdef.modified", format!("{}", version));
            assert_eq!("unknown.gabcdef.modified", format!("{:?}", version));
        }

        #[test]
        fn notontag_notmodified() {
            let version = GitInfo {
                tag_info: Some(TagInfo {
                    tag: "v1.2.3",
                    commits_since_tag: 10,
                }),
                commit_id: "abcdef",
                modified: false,
            };
            assert_eq!("v1.2.3+10.gabcdef", format!("{}", version));
            assert_eq!("v1.2.3+10.gabcdef", format!("{:?}", version));
        }

        #[test]
        fn notontag_modified() {
            let version = GitInfo {
                tag_info: Some(TagInfo {
                    tag: "v1.2.3",
                    commits_since_tag: 10,
                }),
                commit_id: "abcdef",
                modified: true,
            };
            assert_eq!("v1.2.3+10.gabcdef.modified", format!("{}", version));
            assert_eq!("v1.2.3+10.gabcdef.modified", format!("{:?}", version));
        }

        #[test]
        fn ontag_notmodified() {
            let version = GitInfo {
                tag_info: Some(TagInfo {
                    tag: "v1.2.3",
                    commits_since_tag: 0,
                }),
                commit_id: "abcdef",
                modified: false,
            };
            assert_eq!("v1.2.3+0.gabcdef", format!("{}", version));
            assert_eq!("v1.2.3+0.gabcdef", format!("{:?}", version));
        }

        #[test]
        fn ontag_modified() {
            let version = GitInfo {
                tag_info: Some(TagInfo {
                    tag: "v1.2.3",
                    commits_since_tag: 0,
                }),
                commit_id: "abcdef",
                modified: true,
            };
            assert_eq!("v1.2.3+0.gabcdef.modified", format!("{}", version));
            assert_eq!("v1.2.3+0.gabcdef.modified", format!("{:?}", version));
        }
    }
}
