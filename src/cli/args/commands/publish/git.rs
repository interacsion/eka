use super::PublishArgs;
use crate::cli::logging::LogValue;
use clap::Parser;
use gix::{
    discover, object::tree::Entry, remote::find::existing, Commit, ObjectId, ThreadSafeRepository,
};
use manifest::core::Manifest;
use path_clean::PathClean;
use std::{
    fs,
    hash::{Hash, Hasher},
    io::{self, Read},
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Error, Debug)]
enum GitError {
    #[error(transparent)]
    Discover(#[from] discover::Error),
    #[error(transparent)]
    RemotNotFound(#[from] existing::Error),
}

#[derive(Parser)]
#[command(next_help_heading = "Git Options")]
pub struct GitArgs {
    /// The target remote to publish the atom(s) to
    #[arg(long, short = 't', default_value = "origin", name = "TARGET")]
    pub remote: String,
    /// The revision to publish the atom(s) from
    ///
    /// Specifies a revision using Git's extended SHA-1 syntax.
    /// This can be a commit hash, branch name, tag, or a relative
    /// reference like HEAD~3 or master@{yesterday}.
    #[arg(
        long,
        short,
        default_value = "HEAD",
        verbatim_doc_comment,
        name = "REVSPEC"
    )]
    pub spec: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtomId {
    pub manifest: ObjectId,
    pub directory: Option<ObjectId>,
}

struct PublishGitContext<'a> {
    repo: &'a gix::Repository,
    tree: gix::Tree<'a>,
    commit: Commit<'a>,
    remote: gix::Remote<'a>,
}

pub async fn run(repo: ThreadSafeRepository, args: PublishArgs) -> anyhow::Result<()> {
    let repo = repo.to_thread_local();

    let context = PublishGitContext::new(&repo, args.vcs.git).await?;

    let atoms: Vec<AtomId> = if args.recursive {
        todo!();
    } else {
        context.get_ids(args.path)
    };
    tracing::info!(message = ?atoms);

    Ok(())
}

impl Hash for AtomId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.manifest.hash(state);
        if let Some(dir) = &self.directory {
            dir.hash(state);
        }
    }
}

impl<'a> PublishGitContext<'a> {
    async fn new(repo: &'a gix::Repository, args: GitArgs) -> anyhow::Result<Self> {
        let GitArgs { remote, spec } = args;
        let remote = async { repo.find_remote(remote.as_str()).log_err() };

        let commit = async {
            repo.rev_parse_single(spec.as_str())
                .log_err()
                .map(|s| repo.find_commit(s).log_err())
        };

        // print both errors before returning one
        let (remote, commit) = tokio::join!(remote, commit);
        let (remote, commit) = (remote?, commit??);

        let tree = commit.tree()?;

        Ok(Self {
            repo,
            tree,
            commit,
            remote,
        })
    }

    fn get_ids<C>(&self, paths: C) -> Vec<AtomId>
    where
        C: IntoIterator<Item = PathBuf>,
    {
        paths
            .into_iter()
            .filter_map(|path| {
                let atom_path = if matches!(path.extension(), Some(ext) if ext == "atom") {
                    path
                } else {
                    path.with_extension("atom")
                };
                self.repo.work_dir().map_or_else(
                    || self.get_id(&atom_path),
                    |rel_repo| self.get_local_id(rel_repo, &atom_path),
                )
            })
            .collect()
    }

    fn get_id(&self, path: &PathBuf) -> Option<AtomId> {
        let mut exists = false;
        let atom_id = self
            .tree
            .clone()
            .peel_to_entry_by_path(path)
            .ok()
            .flatten()
            .and_then(|entry| {
                self.verify_manifest(&entry, path).or_else(|| {
                    exists = true;
                    None
                })
            });
        let atom_dir_id = self
            .tree
            .clone()
            .peel_to_entry_by_path(path.with_extension(""))
            .ok()
            .flatten()
            .and_then(|entry| entry.mode().is_tree().then_some(entry.oid().to_owned()));
        if let Some(id) = atom_id {
            Some(AtomId {
                manifest: id,
                directory: atom_dir_id,
            })
        } else {
            if !exists {
                tracing::warn!(
                    message = "Atom does not exist in history",
                    path = %path.display(),
                    commit = %self.commit.id(),
                );
            }
            None
        }
    }

    fn get_local_id(&self, rel_repo: &Path, atom_path: &PathBuf) -> Option<AtomId> {
        // unwrap is safe as we won't enter this block when workdir doesn't exist
        let abs_repo = fs::canonicalize(rel_repo).unwrap();
        let current = self.repo.current_dir();
        let rel = current
            .join(atom_path)
            .clean()
            .strip_prefix(&abs_repo)
            .map(Path::to_path_buf);

        rel.or_else(|e| {
            if !atom_path.is_absolute() {
                return Err(e);
            }
            let cleaned = atom_path.clean();
            // Preserve the platform-specific root
            let p = cleaned.strip_prefix(Path::new("/")).unwrap();
            abs_repo
                .join(p)
                .clean()
                .strip_prefix(&abs_repo)
                .map(ToOwned::to_owned)
        })
        .map_err(|e| {
            tracing::warn!(
                message = "Ignoring path outside repo root",
                path = %atom_path.display()
            );
            e
        })
        .map(|path| self.get_id(&path))
        .ok()
        .flatten()
    }

    fn verify_manifest(&self, entry: &Entry, path: &Path) -> Option<ObjectId> {
        if !entry.mode().is_blob() {
            return None;
        }

        let content = read_blob(entry, |reader| {
            let mut content = String::new();
            reader.read_to_string(&mut content)?;
            Ok(content)
        })?;

        Manifest::is(&content)
            .map(|_| entry.oid().to_owned())
            .map_err(|e| {
                tracing::warn!(
                    message = "Ignoring invalid atom manifest",
                    path = %path.display(),
                    commit = %self.commit.id(),
                    oid = %entry.oid(),
                    error = %format!("'{}'", e)
                );
                e
            })
            .ok()
    }
}

fn read_blob<F, R>(entry: &Entry, mut f: F) -> Option<R>
where
    F: FnMut(&mut dyn Read) -> io::Result<R>,
{
    let object = entry.object().ok()?;
    let mut reader = object.data.as_slice();
    f(&mut reader).ok()
}
