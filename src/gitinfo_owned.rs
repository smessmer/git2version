use git2::{Repository, StatusOptions, StatusShow};

use super::git_helpers;

use super::COMMIT_ID_SHORT_HASH_LENGTH;

/// Owned version of [`TagInfo`](crate::TagInfo) containing information about the closest ancestor tag.
///
/// This struct is used during build time when owned strings are needed.
/// For the borrowed version used at runtime, see [`TagInfo`](crate::TagInfo).
#[derive(Clone, PartialEq, Eq)]
pub struct TagInfoOwned {
    /// The name of the tag (e.g., `"v1.2.3"`, `"release-1.0"`).
    pub tag: String,

    /// The number of commits between the tagged commit and the current HEAD.
    /// This is `0` if HEAD is the tagged commit itself.
    pub commits_since_tag: u32,
}

/// Owned version of [`GitInfo`](crate::GitInfo) containing git version information.
///
/// This struct is used during build time by [`get_git_info`] when owned strings
/// are needed. The data is then serialized to environment variables by [`init_proxy_build!`](crate::init_proxy_build)
/// and reconstructed as [`GitInfo`](crate::GitInfo) (with borrowed strings) at compile time
/// by the [`init_proxy_lib!`](crate::init_proxy_lib) macro.
#[derive(Clone, PartialEq, Eq)]
pub struct GitInfoOwned {
    /// Information on the tag that is the closest ancestor tag to the current commit.
    /// This is `None` if the repository has no tags or is a shallow clone where tags
    /// aren't available.
    pub tag_info: Option<TagInfoOwned>,

    /// The shortened ID of the current HEAD commit.
    /// Length is determined by [`COMMIT_ID_SHORT_HASH_LENGTH`](crate::COMMIT_ID_SHORT_HASH_LENGTH).
    pub commit_id: String,

    /// Whether the working directory has uncommitted changes (staged or unstaged).
    /// Untracked files are not considered modifications.
    pub modified: bool,
}

/// Retrieves git version information from the given repository.
///
/// This function is called by the [`init_proxy_build!`](crate::init_proxy_build) macro
/// in the proxy crate's `build.rs` to extract version information from git.
///
/// # Arguments
///
/// * `repo` - A reference to an opened git2 [`Repository`]
///
/// # Returns
///
/// Returns a [`GitInfoOwned`] containing:
/// - The closest ancestor tag (if any) and commits since that tag
/// - The shortened HEAD commit ID (10 characters)
/// - Whether the working directory has modifications
///
/// # Errors
///
/// Returns a [`git2::Error`] if:
/// - The repository HEAD cannot be resolved (e.g., empty repository with no commits)
/// - Git status cannot be retrieved
/// - Tag names contain non-UTF8 characters
///
/// # Tag Resolution
///
/// Tags are resolved by walking the first-parent history (ignoring merge commits)
/// from HEAD until a tagged commit is found. If multiple tags exist on the same
/// commit, the first one encountered is used.
///
/// # Example
///
/// ```ignore
/// use git2::Repository;
/// use git2version::get_git_info;
///
/// let repo = Repository::discover(".").unwrap();
/// let info = get_git_info(&repo).unwrap();
/// println!("Commit: {}", info.commit_id);
/// if let Some(tag_info) = info.tag_info {
///     println!("Tag: {} (+{} commits)", tag_info.tag, tag_info.commits_since_tag);
/// }
/// ```
pub fn get_git_info(repo: &Repository) -> Result<GitInfoOwned, git2::Error> {
    let head_commit = repo.head()?.peel_to_commit()?;
    let head_commit_id_str = head_commit.id().to_string();
    let head_commit_id_str = head_commit_id_str[..COMMIT_ID_SHORT_HASH_LENGTH].to_string();

    let modified = {
        let statuses = repo.statuses(Some(
            StatusOptions::default()
                .show(StatusShow::IndexAndWorkdir)
                .include_untracked(false)
                .include_ignored(false)
                .include_unmodified(false)
                .exclude_submodules(false),
        ))?;
        statuses.iter().any(|status| {
            status.status() != git2::Status::CURRENT && status.status() != git2::Status::IGNORED
        })
    };

    // find closest ancestor tag, only looking at first parents (i.e. ignoring merge commits)
    // We do this without using `git describe` because the `git describe` format can be ambigious
    // if the version number contains dashes
    let all_tags = git_helpers::all_tags(repo)?;
    let mut current_commit = head_commit;
    let mut commits_since_tag = 0;
    loop {
        let commit_id = current_commit.id();
        if let Some(tags) = all_tags.get(&commit_id) {
            // TODO Don't just take the first tag, but compare version numbers
            let tag = tags.first().expect(
                "tag list can't be empty, because the `all_tags` HashMap only contains entries that have at least one element",
            );
            return Ok(GitInfoOwned {
                tag_info: Some(TagInfoOwned {
                    tag: tag.to_string(),
                    commits_since_tag,
                }),
                commit_id: head_commit_id_str,
                modified,
            });
        }
        match current_commit.parent(0) {
            Ok(parent) => current_commit = parent,
            Err(_) => {
                // We reached the root commit without finding a tag
                return Ok(GitInfoOwned {
                    tag_info: None,
                    commit_id: head_commit_id_str,
                    modified,
                });
            }
        }
        commits_since_tag += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;

    fn create_repo(path: &std::path::Path) -> Repository {
        let repo = Repository::init(path).unwrap();
        repo.config()
            .unwrap()
            .set_str("user.name", "Test User")
            .unwrap();
        repo.config()
            .unwrap()
            .set_str("user.email", "test@example.com")
            .unwrap();
        repo
    }

    fn create_initial_commit(repo: &Repository) -> git2::Oid {
        let content = "initial content";
        std::fs::write(repo.workdir().unwrap().join("file.txt"), content).unwrap();

        let mut index = repo.index().unwrap();
        index
            .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();

        let sig = repo.signature().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap()
    }

    fn create_commit(repo: &Repository, content: &str) -> git2::Oid {
        std::fs::write(repo.workdir().unwrap().join("file.txt"), content).unwrap();

        let mut index = repo.index().unwrap();
        index
            .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();

        let sig = repo.signature().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &format!("Commit: {}", content),
            &tree,
            &[&head_commit],
        )
        .unwrap()
    }

    fn create_tag(repo: &Repository, tag_name: &str) {
        let head_commit = repo
            .head()
            .unwrap()
            .peel(git2::ObjectType::Commit)
            .unwrap();
        repo.tag_lightweight(tag_name, &head_commit, true).unwrap();
    }

    fn add_to_index(repo: &Repository) {
        let mut index = repo.index().unwrap();
        index
            .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();
    }

    #[test]
    fn commit_id_has_correct_length() {
        let dir = TempDir::new("test").unwrap();
        let repo = create_repo(dir.path());
        create_initial_commit(&repo);

        let info = get_git_info(&repo).unwrap();
        assert_eq!(info.commit_id.len(), COMMIT_ID_SHORT_HASH_LENGTH);
    }

    #[test]
    fn no_tags_clean_workdir() {
        let dir = TempDir::new("test").unwrap();
        let repo = create_repo(dir.path());
        create_initial_commit(&repo);

        let info = get_git_info(&repo).unwrap();
        assert!(info.tag_info.is_none());
        assert!(!info.modified);
    }

    #[test]
    fn no_tags_dirty_workdir() {
        let dir = TempDir::new("test").unwrap();
        let repo = create_repo(dir.path());
        create_initial_commit(&repo);

        // Modify a tracked file
        std::fs::write(repo.workdir().unwrap().join("file.txt"), "modified").unwrap();

        let info = get_git_info(&repo).unwrap();
        assert!(info.tag_info.is_none());
        assert!(info.modified);
    }

    #[test]
    fn on_tag_not_modified() {
        let dir = TempDir::new("test").unwrap();
        let repo = create_repo(dir.path());
        create_initial_commit(&repo);
        create_tag(&repo, "v1.0.0");

        let info = get_git_info(&repo).unwrap();
        assert!(info.tag_info.is_some());
        let tag_info = info.tag_info.unwrap();
        assert_eq!(tag_info.tag, "v1.0.0");
        assert_eq!(tag_info.commits_since_tag, 0);
        assert!(!info.modified);
    }

    #[test]
    fn commits_after_tag() {
        let dir = TempDir::new("test").unwrap();
        let repo = create_repo(dir.path());
        create_initial_commit(&repo);
        create_tag(&repo, "v1.0.0");

        // Add 3 more commits after the tag
        create_commit(&repo, "second");
        create_commit(&repo, "third");
        create_commit(&repo, "fourth");

        let info = get_git_info(&repo).unwrap();
        assert!(info.tag_info.is_some());
        let tag_info = info.tag_info.unwrap();
        assert_eq!(tag_info.tag, "v1.0.0");
        assert_eq!(tag_info.commits_since_tag, 3);
    }

    #[test]
    fn untracked_files_not_counted_as_modified() {
        let dir = TempDir::new("test").unwrap();
        let repo = create_repo(dir.path());
        create_initial_commit(&repo);

        // Add a new untracked file
        std::fs::write(repo.workdir().unwrap().join("untracked.txt"), "new file").unwrap();

        let info = get_git_info(&repo).unwrap();
        // Untracked files should NOT be considered modifications
        assert!(!info.modified);
    }

    #[test]
    fn staged_changes_count_as_modified() {
        let dir = TempDir::new("test").unwrap();
        let repo = create_repo(dir.path());
        create_initial_commit(&repo);

        // Modify a tracked file and stage it
        std::fs::write(repo.workdir().unwrap().join("file.txt"), "staged changes").unwrap();
        add_to_index(&repo);

        let info = get_git_info(&repo).unwrap();
        assert!(info.modified);
    }

    #[test]
    fn empty_repo_returns_error() {
        let dir = TempDir::new("test").unwrap();
        let repo = create_repo(dir.path());

        // Empty repo has no HEAD, should error
        let result = get_git_info(&repo);
        assert!(result.is_err());
    }

    #[test]
    fn commit_id_is_prefix_of_full_hash() {
        let dir = TempDir::new("test").unwrap();
        let repo = create_repo(dir.path());
        let full_oid = create_initial_commit(&repo);
        let full_hash = full_oid.to_string();

        let info = get_git_info(&repo).unwrap();
        assert!(full_hash.starts_with(&info.commit_id));
    }
}
