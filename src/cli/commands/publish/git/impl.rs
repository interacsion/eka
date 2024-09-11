use crate::cli::logging::LogValue;

use gix_actor::Signature;
use gix_object::tree::Entry as AtomEntry;
use manifest::core::{Atom, Manifest};
use path_clean::PathClean;

use gix::{
    diff::object::Commit as AtomCommit, object::tree::Entry, worktree::object::Tree as AtomTree,
    ObjectId, Reference,
};
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

impl<'a> super::PublishGitContext<'a> {
    /// Method to publish an atom
    pub fn publish_atom(&self, path: &PathBuf) -> Option<()> {
        let no_ext = path.with_extension("");
        let (atom, atom_entry) = self
            .tree
            .clone()
            .peel_to_entry_by_path(path)
            .log_err()
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
            .log_err()
            .ok()
            .flatten()
            .and_then(|entry| entry.mode().is_tree().then_some(entry));

        let trees = self.write_atom_trees(&atom_entry, atom_dir_entry)?;

        self.write_atom_commits(&atom, trees)?
            .write_refs(self.repo, &atom, &no_ext)
            .map(|r| {
                tracing::trace!(message = "Atom published", refs = ?r);
            })
    }

    /// Method to publish an atom relative to the work directory
    pub fn publish_workdir_atom(&self, rel_repo: &Path, atom_path: &PathBuf) -> Option<()> {
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

    /// Method to verify the manifest of an entry
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

    /// Method to write atom tree
    fn write_atom_trees(&self, atom: &Entry, dir: Option<Entry>) -> Option<AtomId> {
        let mut entries: Vec<AtomEntry> = Vec::with_capacity(2);

        let tree = atom_tree(&mut entries, atom);
        let id = self.repo.write_object(tree).log_err().ok();

        Some(AtomId {
            manifest: id?.object().ok()?.id,
            directory: dir.and_then(|entry| {
                let tree = atom_tree(&mut entries, &entry);
                self.repo
                    .write_object(tree)
                    .log_err()
                    .ok()
                    .and_then(|id| Some(id.object().ok()?.id))
            }),
        })
    }

    /// Method to write atom commits
    fn write_atom_commits(
        &self,
        atom: &Atom,
        AtomId {
            manifest,
            directory,
        }: AtomId,
    ) -> Option<CommittedAtom> {
        let write = |obj| {
            Some(
                self.repo
                    .write_object(obj)
                    .log_err()
                    .ok()?
                    .object()
                    .log_err()
                    .ok()?
                    .id,
            )
        };

        let sig = Signature {
            email: EMPTY.into(),
            name: EMPTY.into(),
            time: gix_date::Time {
                seconds: 0,
                offset: 0,
                sign: gix_date::time::Sign::Plus,
            },
        };
        let commit = AtomCommit {
            tree: manifest,
            parents: Vec::new().into(),
            author: sig.clone(),
            committer: sig,
            encoding: None,
            message: format!("{}: {}", atom.id, atom.version).into(),
            extra_headers: vec![
                ("origin".into(), self.commit.id().as_bytes().into()),
                ("version".into(), FORMAT_VERSION.into()),
            ],
        };
        let id = write(commit.clone())?;
        if let Some(tree) = directory {
            let commit = AtomCommit {
                tree,
                parents: vec![id].into(),
                ..commit
            };
            let id = write(commit.clone())?;
            Some(CommittedAtom { commit, id })
        } else {
            Some(CommittedAtom { commit, id })
        }
    }
}

/// Struct to hold the result of writing atom commits
#[derive(Debug, Clone)]
struct CommittedAtom {
    commit: AtomCommit,
    id: ObjectId,
}

impl CommittedAtom {
    /// Method to write references for the committed atom
    fn write_refs<'a>(
        &'a self,
        repo: &'a gix::Repository,
        atom: &Atom,
        ref_path: &Path,
    ) -> Option<AtomReference> {
        let Self { commit, id } = self;
        use gix_ref::transaction::PreviousValue;
        let write = |kind, id| {
            repo.reference(
                format!("refs/atom/{}-{}/{}", ref_path.display(), atom.version, kind),
                id,
                PreviousValue::MustNotExist,
                format!("publish: {}: {}-{}", atom.id, atom.version, kind),
            )
            .log_err()
            .ok()
        };

        Some(if let Some(manifest) = commit.parents.first() {
            AtomReference {
                manifest: write(MANIFEST, *manifest)?,
                source: write(SOURCE, *id),
            }
        } else {
            AtomReference {
                manifest: write(MANIFEST, *id)?,
                source: None,
            }
        })
    }
}

/// Struct to hold the unique identity of an atom given by the Git object ID of the tree(s) of its contents
struct AtomId {
    manifest: ObjectId,
    directory: Option<ObjectId>,
}

/// Struct to hold references for an atom
#[derive(Debug)]
struct AtomReference<'a> {
    manifest: Reference<'a>,
    source: Option<Reference<'a>>,
}

/// Function to read a blob from an entry
fn read_blob<F, R>(entry: &Entry, mut f: F) -> Option<R>
where
    F: FnMut(&mut dyn Read) -> io::Result<R>,
{
    let object = entry.object().ok()?;
    let mut reader = object.data.as_slice();
    f(&mut reader).ok()
}

/// Function to create an atom tree from entries
fn atom_tree(entries: &mut Vec<AtomEntry>, atom: &Entry) -> AtomTree {
    entries.push(AtomEntry {
        mode: atom.mode(),
        filename: atom.filename().into(),
        oid: atom.object_id(),
    });

    AtomTree {
        entries: entries.clone(),
    }
}

const FORMAT_VERSION: &str = "1";
const EMPTY: &str = "";
const SOURCE: &str = "source";
const MANIFEST: &str = "manifest";
