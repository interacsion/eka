use super::PublishArgs;
use clap::Parser;
use gix::{discover, remote::find::existing, Commit, ObjectId, ThreadSafeRepository};
use path_clean::PathClean;
use std::{
    fs,
    hash::{Hash, Hasher},
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
    /// The repositories remote to publish the atom(s) to
    #[arg(long, default_value = "origin")]
    pub remote: String,
    /// The ref to publish the atom(s) from
    #[arg(long, default_value = "HEAD")]
    pub r#ref: String,
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
    r#ref: gix::Reference<'a>,
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
        let GitArgs { remote, r#ref } = args;
        let remote = async {
            repo.find_remote(remote.as_str()).map_err(|e| {
                tracing::error!(error = %format!("'{e}'"));
                e
            })
        };

        let r#ref = async {
            repo.find_reference(r#ref.as_str()).map_err(|e| {
                tracing::error!(error = %format!("'{e}'"), r#ref);
                e
            })
        };

        // print both errors before returning one
        let (remote, r#ref) = tokio::join!(remote, r#ref);
        let (remote, r#ref) = (remote?, r#ref?);

        let commit = {
            let oid = repo.rev_parse_single(r#ref.name())?;
            repo.find_object(oid)?.try_into_commit()
        }?;

        let tree = commit.tree()?;

        Ok(Self {
            repo,
            tree,
            commit,
            remote,
            r#ref,
        })
    }

    fn get_id(&self, path: &PathBuf) -> Option<AtomId> {
        let atom_id = self
            .tree
            .clone()
            .peel_to_entry_by_path(path)
            .ok()
            .flatten()
            .map(|entry| entry.oid().to_owned());
        let atom_dir_id = self
            .tree
            .clone()
            .peel_to_entry_by_path(path.with_extension(""))
            .ok()
            .flatten()
            .and_then(|entry| {
                if entry.mode().is_tree() {
                    Some(entry.oid().to_owned())
                } else {
                    None
                }
            });
        if let Some(id) = atom_id {
            Some(AtomId {
                manifest: id,
                directory: atom_dir_id,
            })
        } else {
            tracing::warn!(
                message = "Atom does not exist in history",
                path = %path.display(),
                commit = %self.commit.id(),
                r#ref = %self.r#ref.name().as_bstr()
            );
            None
        }
    }

    fn get_local_id(&self, rel_repo: &Path, atom_path: &PathBuf) -> Option<AtomId> {
        // unwrap is safe as we won't enter this block when workdir doesn't exist
        let abs_repo = fs::canonicalize(rel_repo).unwrap();
        let current = self.repo.current_dir();
        current.join(atom_path).clean().strip_prefix(&abs_repo).map(Path::to_path_buf).or_else(|e| {
                    if atom_path.is_absolute() {
                        let cleaned = atom_path.clean();
                        // absolute paths always start with `/` (or its platform equivalent), so unwrap is safe
                        let p = cleaned.strip_prefix(Path::new("/")).unwrap();
                        Ok(abs_repo.join(p).clean().strip_prefix(&abs_repo)?.to_path_buf())
                    } else {
                        Err(e)
                    }
                }).map_err(|e| {
                    tracing::warn!(message = "Ignoring path outside repo root", path = %atom_path.display());
                    e
                })
                .map(|path| self.get_id(&path))
                .ok().flatten()
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
}
