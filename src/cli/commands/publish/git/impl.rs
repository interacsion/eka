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
    path::Path,
};

impl<'a> super::PublishGitContext<'a> {
    /// Method to publish an atom
    pub fn publish_atom(&self, path: &Path) -> Option<()> {
        let no_ext = path.with_extension("");
        let (atom, atom_entry) = self
            .tree_search(path)
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
            .tree_search(&no_ext)
            .and_then(|entry| entry.mode().is_tree().then_some(entry));

        let trees = self.write_atom_trees(&atom_entry, atom_dir_entry)?;

        self.write_atom_commits(&atom, trees)?
            .write_refs(self.repo, &atom, &no_ext)
            .map(|r| {
                tracing::trace!(message = "Atom published", refs = ?r);
            })
    }

    /// Method to publish an atom relative to the work directory
    pub fn publish_workdir_atom(&self, rel_repo: &Path, atom_path: &Path) -> Option<()> {
        let abs_repo = fs::canonicalize(rel_repo).log_err().ok()?;
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
            let p = cleaned.strip_prefix(Path::new("/")).log_err()?;
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
        let id = self.write_object(tree)?;

        Some(AtomId {
            manifest: id,
            directory: dir.and_then(|entry| {
                let tree = atom_tree(&mut entries, &entry);
                self.write_object(tree)
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
        let id = self.write_object(commit.clone())?;
        if let Some(tree) = directory {
            let commit = AtomCommit {
                tree,
                parents: vec![id].into(),
                ..commit
            };
            let id = self.write_object(commit.clone())?;
            Some(CommittedAtom { commit, id })
        } else {
            Some(CommittedAtom { commit, id })
        }
    }

    /// Helper function to write an object to the repository
    fn write_object(&self, obj: impl gix_object::WriteTo) -> Option<gix::ObjectId> {
        self.repo
            .write_object(obj)
            .log_err()
            .map(|id| id.detach())
            .ok()
    }

    /// Helper function to return an entry by path from the repo tree
    fn tree_search(&self, path: &Path) -> Option<Entry<'a>> {
        self.tree
            .clone()
            .peel_to_entry_by_path(path)
            .log_err()
            .ok()
            .flatten()
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

/// Helper function to read a blob from an entry
fn read_blob<F, R>(entry: &Entry, mut f: F) -> Option<R>
where
    F: FnMut(&mut dyn Read) -> io::Result<R>,
{
    let object = entry.object().ok()?;
    let mut reader = object.data.as_slice();
    f(&mut reader).ok()
}

/// Helper function to create an atom tree from entries
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
