use super::PublishArgs;
use crate::cli::logging::LogValue;
use clap::Parser;
use gix::diff::object::Commit as AtomCommit;
use gix::worktree::object::Tree as AtomTree;
use gix::{
    discover, object::tree::Entry, remote::find::existing, Commit, ObjectId, Reference, Remote,
    Repository, ThreadSafeRepository, Tree,
};
use gix_actor::Signature;
use gix_object::tree::Entry as AtomEntry;
use manifest::core::{Atom, Manifest};
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
pub(crate) struct GitArgs {
    /// The target remote to publish the atom(s) to
    #[arg(long, short = 't', default_value = "origin", name = "TARGET")]
    remote: String,
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
    spec: String,
}

struct PublishGitContext<'a> {
    repo: &'a Repository,
    tree: Tree<'a>,
    commit: Commit<'a>,
    remote: Remote<'a>,
}

pub(crate) async fn run(repo: ThreadSafeRepository, args: PublishArgs) -> anyhow::Result<()> {
    let repo = repo.to_thread_local();

    let context = PublishGitContext::new(&repo, args.vcs.git).await?;

    let atoms: Vec<Reference> = if args.recursive {
        todo!();
    } else {
        context.publish(args.path)
    };
    tracing::info!(message = ?atoms);

    Ok(())
}

impl<'a> PublishGitContext<'a> {
    async fn new(repo: &'a Repository, args: GitArgs) -> anyhow::Result<Self> {
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

    fn publish<C>(&self, paths: C) -> Vec<Reference>
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
                    || self.publish_atom(&atom_path),
                    |rel_repo| self.publish_workdir_atom(rel_repo, &atom_path),
                )
            })
            .collect()
    }

    fn publish_atom(&self, path: &PathBuf) -> Option<Reference> {
        let no_ext = path.with_extension("");
        let (atom, atom_entry) = self
            .tree
            .clone()
            .peel_to_entry_by_path(path)
            .ok()
            .flatten()
            .or_else(|| {
                tracing::warn!(
                    message = "Atom does not exist in given history",
                    path = %path.display(),
                    commit = %self.commit.id(),
                );
                None
            })
            .and_then(|entry| self.verify_manifest(&entry, path).map(|atom| (atom, entry)))?;
        let atom_dir_entry = self
            .tree
            .clone()
            .peel_to_entry_by_path(&no_ext)
            .ok()
            .flatten()
            .and_then(|entry| entry.mode().is_tree().then_some(entry));
        let tree = self
            .write_atom_tree(&atom_entry, atom_dir_entry)?
            .object()
            .ok()?
            .id;

        let id = self.write_atom_commit(&atom, tree)?;
        use gix_ref::transaction::PreviousValue;
        self.repo
            .reference(
                format!("refs/atom/{}-{}", no_ext.display(), atom.version),
                id,
                PreviousValue::MustNotExist,
                format!("publish: {}: {}", atom.id, atom.version),
            )
            .log_err()
            .ok()
    }

    fn publish_workdir_atom(&self, rel_repo: &Path, atom_path: &PathBuf) -> Option<Reference> {
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
        .map(|path| self.publish_atom(&path))
        .ok()
        .flatten()
    }

    fn verify_manifest(&self, entry: &Entry, path: &Path) -> Option<Atom> {
        if !entry.mode().is_blob() {
            return None;
        }

        let content = read_blob(entry, |reader| {
            let mut content = String::new();
            reader.read_to_string(&mut content)?;
            Ok(content)
        })?;

        Manifest::is(&content)
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

    fn write_atom_tree(&self, atom: &Entry, dir: Option<Entry>) -> Option<gix::Id> {
        let mut entries: Vec<AtomEntry> = Vec::with_capacity(2);
        entries.push(AtomEntry {
            mode: atom.mode(),
            filename: atom.filename().into(),
            oid: atom.object_id(),
        });
        if let Some(entry) = dir {
            entries.push(AtomEntry {
                mode: entry.mode(),
                filename: entry.filename().into(),
                oid: entry.object_id(),
            });
        }
        let tree: AtomTree = AtomTree { entries };
        self.repo.write_object(tree).log_err().ok()
    }

    fn write_atom_commit(&self, atom: &Atom, tree: ObjectId) -> Option<gix::Id> {
        let sig = Signature {
            email: "".into(),
            name: "".into(),
            time: gix_date::Time {
                seconds: 0,
                offset: 0,
                sign: gix_date::time::Sign::Plus,
            },
        };
        let commit = AtomCommit {
            tree,
            parents: Vec::new().into(),
            author: sig.clone(),
            committer: sig,
            encoding: None,
            message: format!("{}: {}", atom.id, atom.version).into(),
            extra_headers: vec![("commit".into(), self.commit.id().as_bytes().into())],
        };
        self.repo.write_object(commit).log_err().ok()
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
