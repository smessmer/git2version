use git2::{Oid, Repository};
use std::collections::hash_map::{Entry, HashMap};

/// Retrieves all tags from a repository, grouped by the commit they point to.
///
/// This function iterates over all tags in the repository and builds a mapping
/// from commit OID to the list of tag names pointing to that commit.
///
/// # Arguments
///
/// * `repo` - A reference to an opened git2 [`Repository`]
///
/// # Returns
///
/// Returns a [`HashMap`] where:
/// - Keys are commit [`Oid`]s
/// - Values are vectors of tag name strings (without the `"refs/tags/"` prefix)
///
/// Multiple tags can point to the same commit, hence the `Vec<String>` value type.
///
/// # Errors
///
/// Returns a [`git2::Error`] if:
/// - Tag iteration fails
/// - A tag name is not valid UTF-8
/// - A tag name doesn't start with `"refs/tags/"` (should not happen with valid repos)
///
/// # Note
///
/// For annotated tags, this function receives the OID of the tag object itself,
/// not the commit it points to. The [`get_git_info`](crate::get_git_info) function
/// handles this by looking up tags during commit traversal.
pub fn all_tags(repo: &Repository) -> Result<HashMap<Oid, Vec<String>>, git2::Error> {
    let mut result: HashMap<Oid, Vec<String>> = HashMap::new();
    // Because `Repository::tag_foreach` doesn't support the callback to return an error, we
    // keep a variable remembering whether an error happened and set it from the callback.
    let mut error = None;
    repo.tag_foreach(|commit_id, name| {
        let name = std::str::from_utf8(name)
            .map_err(|err| git2::Error::from_str(&format!("Tag name is not valid UTF-8: {}", err)));
        let name = match name {
            Ok(name) => name,
            Err(err) => {
                assert!(
                    error.is_none(),
                    "We immediately exit after an error so this can't be set yet"
                );
                // Set error and stop iterating
                error = Some(err);
                return false;
            }
        };
        let name = name.strip_prefix("refs/tags/").ok_or_else(|| {
            git2::Error::from_str(&format!(
                "Tag name '{}' doesn't start with 'refs/tags/'",
                name
            ))
        });
        let name = match name {
            Ok(name) => name,
            Err(err) => {
                assert!(
                    error.is_none(),
                    "We immediately exit after an error so this can't be set yet"
                );
                // Set error and stop iterating
                error = Some(err);
                return false;
            }
        };
        let name = name.to_owned();
        match result.entry(commit_id) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().push(name);
            }
            Entry::Vacant(entry) => {
                entry.insert(vec![name]);
            }
        }
        true
    })?;

    if let Some(error) = error {
        Err(error)
    } else {
        Ok(result)
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

    fn create_initial_commit(repo: &Repository) -> Oid {
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

    fn create_commit(repo: &Repository, content: &str) -> Oid {
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

    fn create_tag(repo: &Repository, tag_name: &str) -> Oid {
        let head_commit = repo
            .head()
            .unwrap()
            .peel(git2::ObjectType::Commit)
            .unwrap();
        repo.tag_lightweight(tag_name, &head_commit, true).unwrap()
    }

    #[test]
    fn repo_with_no_commits_returns_empty_hashmap() {
        let dir = TempDir::new("test").unwrap();
        let repo = create_repo(dir.path());
        let tags = all_tags(&repo).unwrap();
        assert!(tags.is_empty());
    }

    #[test]
    fn repo_with_commits_but_no_tags() {
        let dir = TempDir::new("test").unwrap();
        let repo = create_repo(dir.path());
        create_initial_commit(&repo);
        create_commit(&repo, "second");
        let tags = all_tags(&repo).unwrap();
        assert!(tags.is_empty());
    }

    #[test]
    fn repo_with_single_tag() {
        let dir = TempDir::new("test").unwrap();
        let repo = create_repo(dir.path());
        let commit_oid = create_initial_commit(&repo);
        create_tag(&repo, "v1.0.0");

        let tags = all_tags(&repo).unwrap();
        assert_eq!(tags.len(), 1);
        assert!(tags.contains_key(&commit_oid));
        assert_eq!(tags[&commit_oid], vec!["v1.0.0"]);
    }

    #[test]
    fn multiple_tags_on_different_commits() {
        let dir = TempDir::new("test").unwrap();
        let repo = create_repo(dir.path());

        let first_commit = create_initial_commit(&repo);
        create_tag(&repo, "v1.0.0");

        let second_commit = create_commit(&repo, "second");
        create_tag(&repo, "v2.0.0");

        let tags = all_tags(&repo).unwrap();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[&first_commit], vec!["v1.0.0"]);
        assert_eq!(tags[&second_commit], vec!["v2.0.0"]);
    }

    #[test]
    fn multiple_tags_on_same_commit() {
        let dir = TempDir::new("test").unwrap();
        let repo = create_repo(dir.path());

        let commit_oid = create_initial_commit(&repo);
        create_tag(&repo, "v1.0.0");
        create_tag(&repo, "release-1.0");

        let tags = all_tags(&repo).unwrap();
        assert_eq!(tags.len(), 1);
        assert!(tags.contains_key(&commit_oid));

        let tag_names = &tags[&commit_oid];
        assert_eq!(tag_names.len(), 2);
        assert!(tag_names.contains(&"v1.0.0".to_string()));
        assert!(tag_names.contains(&"release-1.0".to_string()));
    }

    #[test]
    fn tag_names_without_v_prefix() {
        let dir = TempDir::new("test").unwrap();
        let repo = create_repo(dir.path());

        let commit_oid = create_initial_commit(&repo);
        create_tag(&repo, "release-1.0");

        let tags = all_tags(&repo).unwrap();
        assert_eq!(tags[&commit_oid], vec!["release-1.0"]);
    }

    #[test]
    fn tag_with_special_characters() {
        let dir = TempDir::new("test").unwrap();
        let repo = create_repo(dir.path());

        let commit_oid = create_initial_commit(&repo);
        // Tags can contain dots, dashes, and underscores
        create_tag(&repo, "v1.0.0-beta.1_test");

        let tags = all_tags(&repo).unwrap();
        assert_eq!(tags[&commit_oid], vec!["v1.0.0-beta.1_test"]);
    }
}
