mod r#impl;

use super::{error::GitError, Content, PublishOutcome, Record};
use crate::{Atom, AtomId};

use gix::Commit;
use gix::{ObjectId, Repository, Tree};
use std::{cell::RefCell, ops::Deref};
use tokio::task::JoinSet;

pub type GitAtomId = AtomId<Root>;
type GitRecord = Record<Root>;
pub type GitOutcome = PublishOutcome<Root>;
pub type GitResult<R> = Result<R, GitError>;

#[derive(Debug)]
/// Holds the shared context needed for publishing atoms
pub struct GitContext<'a> {
    /// Reference to the repository we are publish from
    repo: &'a Repository,
    /// The repository tree object for the given commit
    tree: Tree<'a>,
    /// The commit to publish from
    commit: Commit<'a>,
    /// Store the given remote name as a &str for convenient use
    remote_str: &'a str,
    /// a JoinSet of push tasks to avoid blocking on them
    push_tasks: RefCell<JoinSet<Result<Vec<u8>, GitError>>>,
}

struct AtomContext<'a> {
    atom: &'a Atom,
    id: &'a GitAtomId,
    path: &'a Path,
    ref_prefix: String,
    git: &'a GitContext<'a>,
}

trait ExtendRepo {
    /// Normalizes a given path to be relative to the repository root.
    ///
    /// This function takes a path (relative or absolute) and attempts to normalize it
    /// relative to the repository root, based on the current working directory within
    /// the repository's file system.
    ///
    /// # Behavior:
    /// - For relative paths (e.g., "foo/bar" or "../foo"):
    ///   - Interpreted as relative to the current working directory within the repository.
    ///   - Computed relative to the repository root.
    ///
    /// - For absolute paths (e.g., "/foo/bar"):
    ///   - Treated as if the repository root is the filesystem root.
    ///   - The leading slash is ignored, and the path is considered relative to the repo root.
    fn normalize(&self, path: &Path) -> Result<PathBuf, GitError>;
}

struct FoundAtom<'a> {
    atom: Atom,
    id: GitAtomId,
    entry: Entry<'a>,
}

use gix::diff::object::Commit as AtomCommit;
use gix::object::tree::Entry;

/// Struct to hold the result of writing atom commits
#[derive(Debug, Clone)]
pub struct CommittedAtom {
    /// the raw structure representing the atom that was successfully committed
    commit: AtomCommit,
    /// The object id of the tip of the atom's history
    tip: ObjectId,
    /// A reference back to the original commit which the blob objects in the atom are referenced from
    src: ObjectId,
}

/// Struct to representing the tree of an atom given by the Git object ID of its contents
struct AtomTreeIds {
    /// the object id of the tree containing only the atom's toml manifest and lock file
    spec: ObjectId,
    /// the object id of the tree representing the optional atom directory, if present
    dir: Option<ObjectId>,
}

enum RefKind {
    Spec,
    Tip,
    Src,
}

struct AtomRef<'a> {
    prefix: &'a str,
    kind: RefKind,
}

use gix::Reference;

#[derive(Debug, Clone)]
/// Struct representing the git refs pointing to the atom's parts
pub(super) struct AtomReferences<'a> {
    /// Git ref pointing to the atom's manifest and lock
    spec: Reference<'a>,
    /// The git ref pointing to the tip of the atom's history
    tip: Reference<'a>,
    /// The git ref pointing to the commit the atom's blob objects are referenced from
    src: Reference<'a>,
}

pub struct GitContent {
    spec: gix::refs::Reference,
    tip: gix::refs::Reference,
    src: gix::refs::Reference,
    path: PathBuf,
    ref_prefix: String,
}

impl GitContent {
    pub fn spec(&self) -> &gix::refs::Reference {
        &self.spec
    }
    pub fn tip(&self) -> &gix::refs::Reference {
        &self.tip
    }
    pub fn src(&self) -> &gix::refs::Reference {
        &self.src
    }
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
    pub fn ref_prefix(&self) -> &String {
        &self.ref_prefix
    }
}

use super::Publish;

impl<'a> Publish<Root> for GitContext<'a> {
    type Error = GitError;

    /// Publishes atoms.
    ///
    /// This function processes a collection of paths, each representing an atom to be published. The publishing
    /// process includes path normalization, existence checks, and actual publishing attempts.
    ///
    /// # Path Normalization
    /// - First attempts to interpret each path as relative to the caller's current location inside the repository.
    /// - If normalization fails (e.g., in a bare repository), falls back to treating the path as already relative to the repo root.
    /// - The normalized path is used to search the Git history, not the file system.
    ///
    /// # Publishing Process
    /// For each path:
    /// 1. Normalizes the path (as described above).
    /// 2. Checks if the atom already exists in the repository.
    ///    - If it exists, the atom is skipped, and a log message is generated.
    /// 3. Attempts to publish the atom.
    ///    - If successful, the atom is added to the repository.
    ///    - If any error occurs during publishing, the atom is skipped, and an error is logged.
    ///
    /// # Error Handling
    /// - The function aims to process all provided paths, even if some fail.
    /// - Errors and skipped atoms are collected as results but do not halt the overall process.
    /// - The function continues until all the atoms have been processed.
    ///
    /// # Return Value
    /// Returns a vector of results types (`Vec<Result<PublishOutcome<T>, Self::Error>>`), where the
    /// outter result represents whether an atom has failed, and the inner result determines whether an
    /// atom was safely skipped, e.g. because it already exists..
    fn publish<C>(&self, paths: C) -> Vec<GitResult<GitOutcome>>
    where
        C: IntoIterator<Item = PathBuf>,
    {
        paths
            .into_iter()
            .map(|path| {
                let path = match self.repo.normalize(&path.with_extension("atom")) {
                    Ok(path) => path,
                    Err(GitError::NoWorkDir) => path,
                    Err(e) => return Err(e),
                };
                self.publish_atom(&path)
            })
            .collect()
    }

    fn publish_atom<P: AsRef<Path>>(&self, path: P) -> GitResult<GitOutcome> {
        use Err as Skipped;
        use Ok as Published;

        let blueprint = path.as_ref();
        let dir = blueprint.with_extension("");

        let FoundAtom { atom, id, entry } = self.find_and_verify_atom(blueprint)?;

        let atom = AtomContext::set(&atom, &id, &dir, self);
        let atom_dir_entry = atom.maybe_dir();

        let tree_ids = match atom.write_atom_trees(&entry, atom_dir_entry)? {
            Ok(t) => t,
            Skipped(id) => return Ok(Skipped(id)),
        };

        let refs = atom
            .write_atom_commits(tree_ids)?
            .write_refs(&atom)?
            .push(&atom);

        Ok(Published(GitRecord {
            id,
            content: Content::Git(refs),
        }))
    }
}

impl<'a> AtomContext<'a> {
    fn set(atom: &'a Atom, id: &'a GitAtomId, path: &'a Path, git: &'a GitContext) -> Self {
        let prefix = format!("{}/{}/{}", ATOM_REF_TOP_LEVEL, id, atom.version);
        Self {
            atom,
            id,
            path,
            ref_prefix: prefix,
            git,
        }
    }
}

impl<'a> GitContext<'a> {
    pub async fn set(repo: &'a Repository, remote_str: &'a str, refspec: &str) -> GitResult<Self> {
        let remote = async { repo.find_remote(remote_str) };

        let commit = async { repo.rev_parse_single(refspec).map(|s| repo.find_commit(s)) };

        // print both errors before returning one
        let (remote, commit) = tokio::join!(remote, commit);
        let (_remote, commit) = (remote.map_err(Box::new)?, commit.map_err(Box::new)??);

        let tree = commit.tree()?;

        let push_tasks = RefCell::new(JoinSet::new());

        Ok(Self {
            repo,
            tree,
            commit,
            remote_str,
            push_tasks,
        })
    }

    pub async fn await_pushes(&self, errors: &mut Vec<GitError>) {
        use tokio::sync::Mutex;

        let tasks = Mutex::new(self.push_tasks.borrow_mut());

        while let Some(task) = tasks.lock().await.join_next().await {
            match task {
                Ok(Ok(output)) => {
                    if !output.is_empty() {
                        tracing::info!(output = %String::from_utf8_lossy(&output));
                    }
                }
                Ok(Err(e)) => {
                    errors.push(e);
                }
                Err(e) => {
                    errors.push(GitError::JoinFailed(e));
                }
            }
        }
    }
}

use std::path::{Path, PathBuf};

impl ExtendRepo for Repository {
    fn normalize(&self, path: &Path) -> Result<PathBuf, GitError> {
        use path_clean::PathClean;
        use std::fs;

        let rel_repo_root = self.work_dir().ok_or(GitError::NoWorkDir)?;
        let repo_root = fs::canonicalize(rel_repo_root)?;
        let current = self.current_dir();
        let rel = current.join(path).clean();

        rel.strip_prefix(&repo_root)
            .map_or_else(
                |e| {
                    // handle absolute paths as if they were relative to the repo root
                    if !path.is_absolute() {
                        return Err(e);
                    }
                    let cleaned = path.clean();
                    // Preserve the platform-specific root
                    let p = cleaned.strip_prefix(Path::new("/"))?;
                    repo_root
                        .join(p)
                        .clean()
                        .strip_prefix(&repo_root)
                        .map(Path::to_path_buf)
                },
                |p| Ok(p.to_path_buf()),
            )
            .map_err(|e| {
                tracing::warn!(
                    message = "Ignoring path outside repo root",
                    path = %path.display(),
                );
                GitError::NormalizationFailed(e)
            })
    }
}

use crate::id::CalculateRoot;

pub struct Root(ObjectId);

impl<'a> CalculateRoot<Root> for Commit<'a> {
    type Error = GitError;
    fn calculate_root(&self) -> GitResult<Root> {
        use gix::traverse::commit::simple::{CommitTimeOrder, Sorting};
        // FIXME: we rely on a custom crate patch to search the commit graph
        // with a bias for older commits. The default gix behavior is the opposite
        // starting with bias for newer commits.
        //
        // it is based on the more general concept of an OldestFirst traversal
        // introduce by @nrdxp upstream: https://github.com/Byron/gitoxide/pull/1610
        //
        // However, that work tracks main and the goal of this patch is to remain
        // as minimal as possible on top of a release tag, for easier maintenance
        // assuming it may take a while to merge upstream.
        let mut walk = self
            .ancestors()
            .use_commit_graph(true)
            .sorting(Sorting::ByCommitTime(CommitTimeOrder::OldestFirst))
            .all()?;

        while let Some(Ok(info)) = walk.next() {
            if info.parent_ids.is_empty() {
                return Ok(Root(info.id));
            }
        }

        Err(GitError::RootNotFound)
    }
}

impl AsRef<[u8]> for Root {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Deref for Root {
    type Target = ObjectId;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The current version of the atom ref format
const ATOM_FORMAT_VERSION: &str = "v1";
const EMPTY: &str = "";
/// the namespace under refs to publish atoms
const ATOM_REF_TOP_LEVEL: &str = "atoms";
const ATOM_TIP_REF: &str = "tip";
const ATOM_SPEC_REF: &str = "spec";
const ATOM_SRC_REF: &str = "src";
