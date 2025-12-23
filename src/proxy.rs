/// Initializes the build script for a git2version proxy crate.
///
/// This macro should be called in the `build.rs` of your version proxy crate.
/// It performs the following operations:
///
/// 1. Discovers the git repository containing the proxy crate
/// 2. Extracts version information (tag, commits since tag, commit ID, modified status)
/// 3. Sets build environment variables for the `init_proxy_lib!` macro to consume
/// 4. Configures cargo to rerun when the repository changes
///
/// # Usage
///
/// In your proxy crate's `build.rs`:
///
/// ```ignore
/// fn main() {
///     git2version::init_proxy_build!();
/// }
/// ```
///
/// # Rerun Behavior
///
/// The build script will rerun when:
/// - Any file in the repository working directory changes (to update the `modified` flag)
/// - Any file in the `.git` directory changes (to detect new tags, commits, fetches, etc.)
///
/// # Errors
///
/// If git information cannot be retrieved (e.g., not in a git repository, empty repo),
/// the macro emits a cargo warning and sets up the build environment so that `init_proxy_lib!`
/// will generate a `GITINFO` constant that is `None`.
///
/// # Requirements
///
/// - Must be called from a `build.rs` script
/// - The proxy crate must have `git2version` with the `build` feature as a build-dependency
///
/// # See Also
///
/// - `init_proxy_lib!` - The companion macro for `lib.rs`
/// - The [crate-level documentation](crate) for full setup instructions
// This needs to be a macro instead of just a function because we need the `CARGO_MANIFEST_DIR`
// of the client library, not our own.
#[macro_export]
macro_rules! init_proxy_build {
    () => {
        use $crate::GitInfo;

        let cargo_manifest_dir = env!("CARGO_MANIFEST_DIR");

        fn output_none() {
            println!("cargo:rustc-env=PACKAGEVERSION_GITVERSION_IS_KNOWN=false");
            println!("cargo:rustc-env=PACKAGEVERSION_GITVERSION_HAS_TAG=false");
            println!("cargo:rustc-env=PACKAGEVERSION_GITVERSION_TAG=");
            println!("cargo:rustc-env=PACKAGEVERSION_GITVERSION_COMMITS_SINCE_TAG=");
            println!("cargo:rustc-env=PACKAGEVERSION_GITVERSION_COMMIT_ID=");
            println!("cargo:rustc-env=PACKAGEVERSION_GITVERSION_MODIFIED=");
        }

        let repo = match $crate::git2::Repository::discover(cargo_manifest_dir) {
            Ok(repo) => Some(repo),
            Err(err) => {
                println!("cargo:warning=Error getting version info from git, didn't find git repository: {}", err);
                None
            }
        };
        let repository_version = repo.as_ref().and_then(|repo|
            match $crate::get_git_info(&repo) {
                Ok(git_info) => Some(git_info),
                Err(err) => {
                    println!("cargo:warning=Error getting version info from git: {}", err);
                    None
                }
            }
        );

        if let Some(repository_version) = repository_version {
            println!("cargo:rustc-env=PACKAGEVERSION_GITVERSION_IS_KNOWN=true");
            if let Some(tag_info) = repository_version.tag_info {
                println!(
                    "cargo:rustc-env=PACKAGEVERSION_GITVERSION_HAS_TAG=true"
                );
                println!(
                    "cargo:rustc-env=PACKAGEVERSION_GITVERSION_TAG={}",
                    tag_info.tag
                );
                println!(
                    "cargo:rustc-env=PACKAGEVERSION_GITVERSION_COMMITS_SINCE_TAG={}",
                    tag_info.commits_since_tag
                );
            } else {
                println!(
                    "cargo:rustc-env=PACKAGEVERSION_GITVERSION_HAS_TAG=false",
                );
                println!(
                    "cargo:rustc-env=PACKAGEVERSION_GITVERSION_TAG=",
                );
                println!(
                    "cargo:rustc-env=PACKAGEVERSION_GITVERSION_COMMITS_SINCE_TAG=",
                );
            }
            println!(
                "cargo:rustc-env=PACKAGEVERSION_GITVERSION_COMMIT_ID={}",
                repository_version.commit_id
            );
            println!(
                "cargo:rustc-env=PACKAGEVERSION_GITVERSION_MODIFIED={}",
                repository_version.modified
            );
        } else {
            output_none();
        }

        if let Some(repo) = repo {
            // Rerun the build script if any files changed. This is necessary to correctly update
            // the `.modified` flag of version numbers
            println!(
                "cargo:rerun-if-changed={repo_workspace_path}",
                repo_workspace_path = repo.workdir().unwrap().display()
            );

            // Also rerun the build script if anything in the .git repository changed.
            // This is for the case where our `Cargo.toml` is in a subdirectory of the
            // main git repository. In this case, we still need to react to changes in
            // the git repository.
            println!(
                "cargo:rerun-if-changed={repo_path}",
                repo_path = repo.path().display()
            );
        } else {
            // We didn't find a git repository. Let's rerun if the directory of the `Cargo.toml`
            // changed to check if a git repository got added. Note: This won't catch cases where
            // a git repository is added as a parent directory, but probably nothing we can do
            // about that.
            println!(
                "cargo:rerun-if-changed={cargo_manifest_dir}",
            );
        }
    };
}

/// Initializes the library portion of a git2version proxy crate.
///
/// This macro should be called in the `src/lib.rs` of your version proxy crate.
/// It generates a `GITINFO` constant containing the git version information
/// that was extracted at build time by `init_proxy_build!`.
///
/// # Generated Items
///
/// This macro generates:
///
/// - `pub const GITINFO: Option<GitInfo>` - The version information constant
/// - Re-exports all public items from `git2version` (via `pub use git2version::*`)
///
/// # Usage
///
/// In your proxy crate's `src/lib.rs`:
///
/// ```ignore
/// git2version::init_proxy_lib!();
/// ```
///
/// Then in your main crate:
///
/// ```ignore
/// fn main() {
///     if let Some(info) = version_proxy::GITINFO {
///         println!("Version: {}", info);
///     } else {
///         println!("Version: unknown");
///     }
/// }
/// ```
///
/// # The `GITINFO` Constant
///
/// The generated constant has type `Option<GitInfo<'static, 'static>>` where:
///
/// - `Some(GitInfo { ... })` - Git information was successfully retrieved at build time
/// - `None` - Git information could not be retrieved (not in a repo, empty repo, errors, etc.)
///
/// # Display Format
///
/// When converted to a string via [`Display`](std::fmt::Display), `GitInfo` produces
/// output in the format: `{tag}+{commits}.g{commit_id}[.modified]`
///
/// Examples:
/// - `v1.2.3+0.gabcdef1234` - On tag v1.2.3, clean working directory
/// - `v1.2.3+5.gabcdef1234.modified` - 5 commits after v1.2.3, uncommitted changes
/// - `unknown.gabcdef1234` - No ancestor tag found
///
/// # Requirements
///
/// - The `init_proxy_build!` macro must be called in the corresponding `build.rs`
/// - The proxy crate must have `git2version` as a regular dependency
///
/// # See Also
///
/// - `init_proxy_build!` - The companion macro for `build.rs`
/// - `GitInfo` - The struct containing version information
/// - The [crate-level documentation](crate) for full setup instructions
#[macro_export]
macro_rules! init_proxy_lib {
    () => {
        pub use $crate::*;

        /// Git version information extracted at build time.
        ///
        /// This constant contains structured information about the git repository state
        /// when the crate was built, including:
        /// - The closest ancestor tag (if any) and number of commits since that tag
        /// - The shortened commit ID (10 characters)
        /// - Whether the working directory had uncommitted changes
        ///
        /// # Value
        ///
        /// - `Some(GitInfo { ... })` - Git information was successfully retrieved
        /// - `None` - Git information could not be retrieved (not in a git repository,
        ///   empty repository, or an error occurred)
        ///
        /// # Display Format
        ///
        /// When converted to a string, the format is: `{tag}+{commits}.g{commit_id}[.modified]`
        ///
        /// Examples:
        /// - `v1.2.3+0.gabcdef1234` - On tag v1.2.3, clean working directory
        /// - `v1.2.3+5.gabcdef1234.modified` - 5 commits after tag, with local changes
        /// - `unknown.gabcdef1234` - No ancestor tag found
        ///
        /// # Example
        ///
        /// ```ignore
        /// if let Some(info) = GITINFO {
        ///     println!("Version: {}", info);
        ///     println!("Commit: {}", info.commit_id);
        ///     if info.modified {
        ///         println!("Warning: built from modified source");
        ///     }
        /// }
        /// ```
        pub const GITINFO: Option<$crate::GitInfo> = if $crate::konst::result::unwrap!(
            $crate::konst::primitive::parse_bool(env!("PACKAGEVERSION_GITVERSION_IS_KNOWN"))
        ) {
            Some($crate::GitInfo {
                tag_info: if $crate::konst::result::unwrap!($crate::konst::primitive::parse_bool(
                    env!("PACKAGEVERSION_GITVERSION_HAS_TAG")
                )) {
                    Some($crate::TagInfo {
                        tag: env!("PACKAGEVERSION_GITVERSION_TAG"),
                        commits_since_tag: $crate::konst::result::unwrap!({
                            let mut parser = $crate::konst::parsing::Parser::new(env!(
                                "PACKAGEVERSION_GITVERSION_COMMITS_SINCE_TAG"
                            ));
                            parser.parse_u32()
                        }),
                    })
                } else {
                    None
                },
                commit_id: env!("PACKAGEVERSION_GITVERSION_COMMIT_ID"),
                modified: $crate::konst::result::unwrap!($crate::konst::primitive::parse_bool(
                    env!("PACKAGEVERSION_GITVERSION_MODIFIED")
                )),
            })
        } else {
            None
        };
    };
}
