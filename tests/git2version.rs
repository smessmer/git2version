use git2::{Commit, Repository};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempdir::TempDir;

use git2version::{COMMIT_ID_SHORT_HASH_LENGTH, GitInfo, TagInfo};

const FILENAME: &str = "some_file";

// TODO Use indoc! for multiline strings

fn create_repo(path: &Path) -> Repository {
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

fn create_initial_commit(repo: &Repository) {
    create_change(repo);
    add_all_changes_to_index(repo);
    commit(repo, &[], "Initial commit");
}

fn create_change(repo: &Repository) -> String {
    let content = rand::random::<u64>().to_string();
    std::fs::write(repo.workdir().unwrap().join(FILENAME), &content).unwrap();
    content
}

fn add_all_changes_to_index(repo: &Repository) {
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
}

fn commit(repo: &Repository, parents: &[&Commit], description: &str) -> git2::Oid {
    let sig = repo.signature().unwrap();
    let tree_id = {
        let mut index = repo.index().unwrap();
        index.write_tree().unwrap()
    };
    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, description, &tree, parents)
        .unwrap()
}

fn create_change_and_commit(repo: &Repository) -> git2::Oid {
    let content = create_change(repo);

    add_all_changes_to_index(repo);
    let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
    commit(
        repo,
        &[&head_commit],
        &format!("Commit {FILENAME}: {content}"),
    )
}

fn create_tag(repo: &Repository, tag: &str) {
    let head_commit = repo.head().unwrap().peel(git2::ObjectType::Commit).unwrap();
    repo.tag_lightweight(tag, &head_commit, true).unwrap();
}

fn create_some_commits_but_no_tags(repo: &Repository) {
    create_initial_commit(repo);
    create_change_and_commit(repo);
    create_change_and_commit(repo);
}

fn create_some_commits_and_a_tag(repo: &Repository, tag: &str) {
    create_initial_commit(repo);
    create_change_and_commit(repo);
    create_change_and_commit(repo);
    create_tag(repo, tag);
}

fn create_some_commits_a_tag_and_some_more_commits(repo: &Repository, tag: &str) {
    create_initial_commit(repo);
    create_change_and_commit(repo);
    create_change_and_commit(repo);
    create_tag(repo, tag);
    create_change_and_commit(repo);
    create_change_and_commit(repo);
    create_change_and_commit(repo);
    create_change_and_commit(repo);
    create_change_and_commit(repo);
}

#[test]
fn no_git() {
    let project_dir = make_version_test_project();
    run_version_test_project(project_dir.path(), None);
}

#[test]
fn empty_git() {
    let project_dir = make_version_test_project();
    create_repo(project_dir.path());
    run_version_test_project(project_dir.path(), None);
}

#[test]
fn with_initial_commit_notmodified() {
    let project_dir = make_version_test_project();
    let repo = create_repo(project_dir.path());
    create_initial_commit(&repo);
    run_version_test_project(
        project_dir.path(),
        Some(GitInfo {
            tag_info: None,
            commit_id: &head_commit_id(&repo),
            modified: false,
        }),
    );
}

#[test]
fn with_initial_commit_modified() {
    let project_dir = make_version_test_project();
    let repo = create_repo(project_dir.path());
    create_initial_commit(&repo);
    create_change(&repo);
    run_version_test_project(
        project_dir.path(),
        Some(GitInfo {
            tag_info: None,
            commit_id: &head_commit_id(&repo),
            modified: true,
        }),
    );
}

#[test]
fn with_initial_commit_modified_staged() {
    let project_dir = make_version_test_project();
    let repo = create_repo(project_dir.path());
    create_initial_commit(&repo);
    create_change(&repo);
    add_all_changes_to_index(&repo);
    run_version_test_project(
        project_dir.path(),
        Some(GitInfo {
            tag_info: None,
            commit_id: &head_commit_id(&repo),
            modified: true,
        }),
    );
}

#[test]
fn with_some_commits_but_no_tags_notmodified() {
    let project_dir = make_version_test_project();
    let repo = create_repo(project_dir.path());
    create_some_commits_but_no_tags(&repo);
    run_version_test_project(
        project_dir.path(),
        Some(GitInfo {
            tag_info: None,
            commit_id: &head_commit_id(&repo),
            modified: false,
        }),
    );
}

#[test]
fn with_some_commits_but_no_tags_modified() {
    let project_dir = make_version_test_project();
    let repo = create_repo(project_dir.path());
    create_some_commits_but_no_tags(&repo);
    create_change(&repo);
    run_version_test_project(
        project_dir.path(),
        Some(GitInfo {
            tag_info: None,
            commit_id: &head_commit_id(&repo),
            modified: true,
        }),
    );
}

#[test]
fn with_some_commits_but_no_tags_modified_staged() {
    let project_dir = make_version_test_project();
    let repo = create_repo(project_dir.path());
    create_some_commits_but_no_tags(&repo);
    create_change(&repo);
    add_all_changes_to_index(&repo);
    run_version_test_project(
        project_dir.path(),
        Some(GitInfo {
            tag_info: None,
            commit_id: &head_commit_id(&repo),
            modified: true,
        }),
    );
}

#[test]
fn on_tag_notmodified() {
    let project_dir = make_version_test_project();
    let repo = create_repo(project_dir.path());
    create_some_commits_and_a_tag(&repo, "v1.2.3-alpha");
    run_version_test_project(
        project_dir.path(),
        Some(GitInfo {
            tag_info: Some(TagInfo {
                tag: "v1.2.3-alpha",
                commits_since_tag: 0,
            }),
            commit_id: &head_commit_id(&repo),
            modified: false,
        }),
    );
}

#[test]
fn on_tag_modified() {
    let project_dir = make_version_test_project();
    let repo = create_repo(project_dir.path());
    create_some_commits_and_a_tag(&repo, "v1.2.3-alpha");
    create_change(&repo);
    run_version_test_project(
        project_dir.path(),
        Some(GitInfo {
            tag_info: Some(TagInfo {
                tag: "v1.2.3-alpha",
                commits_since_tag: 0,
            }),
            commit_id: &head_commit_id(&repo),
            modified: true,
        }),
    );
}

#[test]
fn on_tag_modified_staged() {
    let project_dir = make_version_test_project();
    let repo = create_repo(project_dir.path());
    create_some_commits_and_a_tag(&repo, "v1.2.3-alpha");
    create_change(&repo);
    add_all_changes_to_index(&repo);
    run_version_test_project(
        project_dir.path(),
        Some(GitInfo {
            tag_info: Some(TagInfo {
                tag: "v1.2.3-alpha",
                commits_since_tag: 0,
            }),
            commit_id: &head_commit_id(&repo),
            modified: true,
        }),
    );
}

#[test]
fn after_tag_notmodified() {
    let project_dir = make_version_test_project();
    let repo = create_repo(project_dir.path());
    create_some_commits_a_tag_and_some_more_commits(&repo, "v1.2.3-alpha");
    run_version_test_project(
        project_dir.path(),
        Some(GitInfo {
            tag_info: Some(TagInfo {
                tag: "v1.2.3-alpha",
                commits_since_tag: 5,
            }),
            commit_id: &head_commit_id(&repo),
            modified: false,
        }),
    );
}

#[test]
fn after_tag_modified() {
    let project_dir = make_version_test_project();
    let repo = create_repo(project_dir.path());
    create_some_commits_a_tag_and_some_more_commits(&repo, "v1.2.3-alpha");
    create_change(&repo);
    run_version_test_project(
        project_dir.path(),
        Some(GitInfo {
            tag_info: Some(TagInfo {
                tag: "v1.2.3-alpha",
                commits_since_tag: 5,
            }),
            commit_id: &head_commit_id(&repo),
            modified: true,
        }),
    );
}

#[test]
fn after_tag_modified_staged() {
    let project_dir = make_version_test_project();
    let repo = create_repo(project_dir.path());
    create_some_commits_a_tag_and_some_more_commits(&repo, "v1.2.3-alpha");
    create_change(&repo);
    add_all_changes_to_index(&repo);
    run_version_test_project(
        project_dir.path(),
        Some(GitInfo {
            tag_info: Some(TagInfo {
                tag: "v1.2.3-alpha",
                commits_since_tag: 5,
            }),
            commit_id: &head_commit_id(&repo),
            modified: true,
        }),
    );
}

// TODO Test that incremental compiles pick up changes, both changes in the git repo (e.g. create tag) and in the source (e.g. .dirty)

// Edge case tests

#[test]
fn detached_head_state() {
    let project_dir = make_version_test_project();
    let repo = create_repo(project_dir.path());
    create_initial_commit(&repo);
    let first_commit_id = head_commit_id(&repo);
    create_change_and_commit(&repo);
    create_tag(&repo, "v1.0.0");
    create_change_and_commit(&repo);

    // Checkout the first commit (detached HEAD) with a clean working directory
    let first_commit = repo
        .find_commit(repo.revparse_single(&first_commit_id).unwrap().id())
        .unwrap();
    repo.set_head_detached(first_commit.id()).unwrap();
    // Reset working directory to match the commit
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();

    // In detached HEAD state, the tag is ahead of us, so no tag_info
    run_version_test_project(
        project_dir.path(),
        Some(GitInfo {
            tag_info: None,
            commit_id: &first_commit_id,
            modified: false,
        }),
    );
}

#[test]
fn multiple_tags_on_same_commit() {
    let project_dir = make_version_test_project();
    let repo = create_repo(project_dir.path());
    create_some_commits_but_no_tags(&repo);
    create_tag(&repo, "v1.0.0");
    create_tag(&repo, "release-1.0");

    // When multiple tags exist on the same commit, one of them should be returned
    // (the order is implementation-defined)
    let output = _run_process(
        Command::new(env!("CARGO"))
            .arg("run")
            .current_dir(project_dir.path()),
    );
    let actual_version: Option<GitInfo> = serde_json::from_str(&output).unwrap();

    assert!(actual_version.is_some());
    let info = actual_version.unwrap();
    assert!(info.tag_info.is_some());
    let tag_info = info.tag_info.unwrap();
    // Should be one of the two tags
    assert!(
        tag_info.tag == "v1.0.0" || tag_info.tag == "release-1.0",
        "Expected tag to be 'v1.0.0' or 'release-1.0', got '{}'",
        tag_info.tag
    );
    assert_eq!(tag_info.commits_since_tag, 0);
}

#[test]
fn tag_with_special_characters() {
    let project_dir = make_version_test_project();
    let repo = create_repo(project_dir.path());
    create_some_commits_but_no_tags(&repo);
    // Tag with semver-like special characters (dashes, dots, underscores)
    create_tag(&repo, "v1.0.0-beta.1_test");
    run_version_test_project(
        project_dir.path(),
        Some(GitInfo {
            tag_info: Some(TagInfo {
                tag: "v1.0.0-beta.1_test",
                commits_since_tag: 0,
            }),
            commit_id: &head_commit_id(&repo),
            modified: false,
        }),
    );
}

#[test]
fn tag_without_v_prefix() {
    let project_dir = make_version_test_project();
    let repo = create_repo(project_dir.path());
    create_some_commits_but_no_tags(&repo);
    // Tag without the common 'v' prefix
    create_tag(&repo, "1.0.0");
    run_version_test_project(
        project_dir.path(),
        Some(GitInfo {
            tag_info: Some(TagInfo {
                tag: "1.0.0",
                commits_since_tag: 0,
            }),
            commit_id: &head_commit_id(&repo),
            modified: false,
        }),
    );
}

#[test]
fn nested_proxy_crate_in_subdirectory() {
    // Test that a proxy crate in a nested subdirectory still finds the git repo
    let dir = TempDir::new("package-version-test-nested").unwrap();
    let dir_path = dir.path();
    let path_to_git2version_crate = env!("CARGO_MANIFEST_DIR");

    // Initialize git repo at the root
    let repo = create_repo(dir_path);
    create_initial_commit(&repo);
    create_tag(&repo, "v2.0.0");

    // Create project structure with deeply nested proxy crate
    create_file(
        &dir_path.join("Cargo.toml"),
        r#"
[package]
name = "nested-version-test"
edition = "2021"
version = "0.1.0"

[workspace]

[dependencies]
version-proxy = {path = "./packages/internal/version-proxy"}
serde_json = "^1.0.96"
        "#,
    );

    create_file(
        &dir_path.join("src/main.rs"),
        r#"
fn main() {
    println!("{}", serde_json::to_string(&version_proxy::GITINFO).unwrap());
}
        "#,
    );

    create_file(
        &dir_path.join("packages/internal/version-proxy/Cargo.toml"),
        &format!(
            r#"
[package]
name = "version-proxy"
edition = "2021"
version = "0.0.0"

[dependencies]
git2version = {{path = "{path_to_git2version_crate}"}}

[build-dependencies]
git2version = {{path = "{path_to_git2version_crate}", features=["build"]}}
        "#
        ),
    );

    create_file(
        &dir_path.join("packages/internal/version-proxy/build.rs"),
        r#"
fn main() {{
    git2version::init_proxy_build!();
}}
        "#,
    );

    create_file(
        &dir_path.join("packages/internal/version-proxy/src/lib.rs"),
        r#"
            git2version::init_proxy_lib!();
        "#,
    );

    let output = _run_process(Command::new(env!("CARGO")).arg("run").current_dir(dir_path));

    let actual_version: Option<GitInfo> = serde_json::from_str(&output).unwrap();
    assert_eq!(
        actual_version,
        Some(GitInfo {
            tag_info: Some(TagInfo {
                tag: "v2.0.0",
                commits_since_tag: 0,
            }),
            commit_id: &head_commit_id(&repo),
            modified: false,
        })
    );
}

fn head_commit_id(repo: &Repository) -> String {
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    let commit_id = head.id().to_string();
    commit_id[..COMMIT_ID_SHORT_HASH_LENGTH].to_string()
}

fn make_version_test_project() -> TempDir {
    let dir = TempDir::new("package-version-test").unwrap();
    let dir_path = dir.path();
    let path_to_git2version_crate = env!("CARGO_MANIFEST_DIR");

    create_file(
        &dir_path.join("Cargo.toml"),
        r#"
[package]
authors = ["Sebastian Messmer <messmer@cryfs.org>"]
name = "package-version-test"
edition = "2021"
version = "0.1.0"

[workspace]

[dependencies]
version-proxy = {path = "./version-proxy"}
serde_json = "^1.0.96"
        "#,
    );

    create_file(
        &dir_path.join("src/main.rs"),
        r#"
fn main() {
    println!("{}", serde_json::to_string(&version_proxy::GITINFO).unwrap());
}
        "#,
    );

    create_file(
        &dir_path.join("version-proxy/Cargo.toml"),
        &format!(
            r#"
[package]
name = "version-proxy"
edition = "2021"
# The version field here is ignored, no need to change it
version = "0.0.0"

[dependencies]
git2version = {{path = "{path_to_git2version_crate}"}}

[build-dependencies]
git2version = {{path = "{path_to_git2version_crate}", features=["build"]}}
        "#
        ),
    );

    create_file(
        &dir_path.join("version-proxy/build.rs"),
        r#"
fn main() {{
    git2version::init_proxy_build!();
}}
        "#,
    );

    create_file(
        &dir_path.join("version-proxy/src/lib.rs"),
        r#"
            git2version::init_proxy_lib!();
        "#,
    );

    dir
}

fn create_file(path: &Path, content: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    File::create(path)
        .unwrap()
        .write_all(content.as_bytes())
        .unwrap();
}

fn run_version_test_project(project_dir: &Path, expected_version: Option<GitInfo>) {
    let output = _run_process(
        Command::new(env!("CARGO"))
            .arg("run")
            .current_dir(project_dir),
    );

    let actual_version: Option<GitInfo> = serde_json::from_str(&output).unwrap();
    assert_eq!(expected_version, actual_version);
}

fn _run_process(cmd: &mut Command) -> String {
    let output = cmd.output().unwrap();
    if !output.status.success() {
        panic!(
            "Command {:?} failed with status {:?} and stdin:\n{}\n\nstderr:\n{}",
            cmd,
            output.status,
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stderr),
        );
    }
    String::from_utf8_lossy(&output.stdout).to_string()
}
