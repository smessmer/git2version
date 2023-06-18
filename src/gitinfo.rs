use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug, Display, Formatter};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagInfo<'a> {
    /// The name of the tag
    pub tag: &'a str,

    /// The number of commits since the tag
    pub commits_since_tag: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct GitInfo<'a, 'b> {
    /// Information on the tag that is the closest ancestor tag to the current commit.
    /// tag_info can be `None` if the repository doesn't have any tags or is a shallow clone and tags aren't available
    pub tag_info: Option<TagInfo<'a>>,

    /// ID of the current commit
    pub commit_id: &'b str,

    /// Whether the working directory was modified or whether the build was done from a clean working directory
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
