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
    pub(super) fn publish_atom(&self, path: &PathBuf) -> Option<Reference> {
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

    pub(super) fn publish_workdir_atom(
        &self,
        rel_repo: &Path,
        atom_path: &PathBuf,
    ) -> Option<Reference> {
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
