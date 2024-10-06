use super::{
    super::{ATOM_FORMAT_VERSION, ATOM_MANIFEST, ATOM_ORIGIN, EMPTY_SIG},
    AtomContext, AtomRef, GitContext, GitResult, RefKind,
};
use crate::{publish::error::GitError, store::git, Atom, AtomId, Manifest};

use gix::{
    actor::Signature,
    diff::object::Commit as AtomCommit,
    object::tree::Entry,
    objs::{tree::Entry as AtomEntry, WriteTo},
    worktree::object::Tree as AtomTree,
    ObjectId, Reference,
};
use std::{
    io::{self, Read},
    path::Path,
};

impl<'a> GitContext<'a> {
    /// Method to verify the manifest of an entry
    pub(super) fn verify_manifest(&self, obj: &Object, path: &Path) -> GitResult<Atom> {
        let content = read_blob(obj, |reader| {
            let mut content = String::new();
            reader.read_to_string(&mut content)?;
            Ok(content)
        })?;

        Manifest::get_atom(&content).map_err(|e| GitError::Invalid(e, Box::new(path.into())))
    }

    /// Compute the ObjectId of the given object without writing it to the repo
    fn compute_hash(&self, obj: &dyn WriteTo) -> Option<ObjectId> {
        use gix::objs;
        use std::io::Cursor;

        let mut buf = Vec::new();
        let mut cursor = Cursor::new(&mut buf);

        obj.write_to(&mut cursor).ok()?;

        let oid = objs::compute_hash(self.repo.object_hash(), obj.kind(), buf.as_slice());

        Some(oid)
    }

    /// Helper function to write an object to the repository
    fn write_object(&self, obj: impl WriteTo) -> GitResult<gix::ObjectId> {
        Ok(self.repo.write_object(obj).map(|id| id.detach())?)
    }

    /// Helper function to return an entry by path from the repo tree
    pub fn tree_search(&self, path: &Path) -> GitResult<Option<Entry<'a>>> {
        Ok(self.tree.clone().peel_to_entry_by_path(path)?)
    }

    pub(super) fn find_and_verify_atom(&self, path: &Path) -> GitResult<FoundAtom> {
        let entry = self
            .tree_search(path)?
            .ok_or(GitError::NotAFile(path.into()))?;

        if !entry.mode().is_blob() {
            return Err(GitError::NotAFile(path.into()));
        }

        self.verify_manifest(&entry.object()?, path)
            .and_then(|atom| {
                let id = AtomId::compute(&self.commit, atom.id.clone())?;
                Ok(FoundAtom { atom, id, entry })
            })
    }
}

use semver::Version;

impl<'a> AtomRef<'a> {
    fn new(kind: RefKind, prefix: &'a str, version: &'a Version) -> Self {
        AtomRef {
            prefix,
            kind,
            version,
        }
    }
}

use std::fmt;

impl<'a> fmt::Display for AtomRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            RefKind::Manifest => write!(f, "{}/{}s/{}", self.prefix, ATOM_MANIFEST, self.version),
            RefKind::Content => write!(f, "{}/{}", self.prefix, self.version),
            RefKind::Source => write!(f, "{}/{}s/{}", self.prefix, ATOM_ORIGIN, self.version),
        }
    }
}

use crate::publish::MaybeSkipped;

impl<'a> AtomContext<'a> {
    fn refs(&self, kind: RefKind) -> AtomRef {
        AtomRef::new(kind, &self.ref_prefix, &self.atom.version)
    }

    pub(super) fn maybe_dir(&self) -> Option<Entry> {
        match self.git.tree_search(self.path).ok()? {
            Some(entry) => entry.mode().is_tree().then_some(entry),
            _ => None,
        }
    }

    fn ref_exists(&self, tree: &AtomTree, atom_ref: AtomRef) -> bool {
        let id = self.git.compute_hash(tree);
        if let Some(id) = id {
            self.git.repo.find_tree(id).is_ok()
                && self.git.repo.find_reference(&atom_ref.to_string()).is_ok()
        } else {
            false
        }
    }
    /// Method to write the atom tree object
    pub(super) fn write_atom_trees(
        &self,
        atom: &Entry,
        dir: Option<Entry>,
    ) -> GitResult<MaybeSkipped<AtomTreeIds>> {
        use Err as Skipped;
        use Ok as Wrote;

        let mut entries: Vec<AtomEntry> = Vec::with_capacity(2);

        let spec_tree = atom_tree(&mut entries, atom);

        let spec_exists = self.ref_exists(&spec_tree, self.refs(RefKind::Manifest));

        if dir.is_none() && spec_exists {
            return Ok(Skipped(self.atom.id.clone()));
        }

        if let Some(entry) = dir {
            let dir_tree = atom_tree(&mut entries, &entry);
            if self.ref_exists(&dir_tree, self.refs(RefKind::Content)) && spec_exists {
                return Ok(Skipped(self.atom.id.clone()));
            }
            let spec = self.git.write_object(spec_tree)?;
            let dir = Some(self.git.write_object(dir_tree)?);
            Ok(Wrote(AtomTreeIds { spec, dir }))
        } else {
            let spec = self.git.write_object(spec_tree)?;
            Ok(Wrote(AtomTreeIds { spec, dir: None }))
        }
    }

    /// Method to write atom commits
    pub(super) fn write_atom_commits(
        &self,
        AtomTreeIds { spec, dir }: AtomTreeIds,
    ) -> GitResult<CommittedAtom> {
        let sig = Signature {
            email: EMPTY_SIG.into(),
            name: EMPTY_SIG.into(),
            time: gix::date::Time {
                seconds: 0,
                offset: 0,
                sign: gix::date::time::Sign::Plus,
            },
        };
        let commit = AtomCommit {
            tree: spec,
            parents: Vec::new().into(),
            author: sig.clone(),
            committer: sig,
            encoding: None,
            message: format!("{}: {}", self.atom.id, self.atom.version).into(),
            extra_headers: vec![
                (
                    "path".into(),
                    self.path.to_string_lossy().to_string().into(),
                ),
                ("root".into(), self.id.root().to_hex().to_string().into()),
                (
                    ATOM_ORIGIN.into(),
                    self.git.commit.id().to_hex().to_string().into(),
                ),
                ("format".into(), ATOM_FORMAT_VERSION.into()),
            ],
        };
        let src = self.git.commit.id;
        let tip = self.git.write_object(commit.clone())?;
        if let Some(tree) = dir {
            let commit = AtomCommit {
                tree,
                parents: vec![tip].into(),
                ..commit
            };
            let tip = self.git.write_object(commit.clone())?;
            Ok(CommittedAtom { commit, tip, src })
        } else {
            Ok(CommittedAtom { commit, tip, src })
        }
    }
}

use super::{CommittedAtom, FoundAtom};

impl<'a> CommittedAtom {
    /// Method to write a single reference to the repository
    fn write_ref(
        &'a self,
        atom: &'a AtomContext,
        id: ObjectId,
        atom_ref: AtomRef,
    ) -> GitResult<Reference> {
        use gix::refs::transaction::PreviousValue;

        tracing::trace!("writing atom ref: {}", atom_ref);

        Ok(atom.git.repo.reference(
            format!("refs/{}", atom_ref),
            id,
            PreviousValue::MustNotExist,
            format!(
                "publish: {}: {}-{}",
                atom.atom.id, atom.atom.version, atom_ref
            ),
        )?)
    }
    /// Method to write references for the committed atom
    pub(super) fn write_refs(&'a self, atom: &'a AtomContext) -> GitResult<AtomReferences> {
        let Self { commit, tip, src } = self;

        Ok(if let Some(spec) = commit.parents.first() {
            AtomReferences {
                manifest: self.write_ref(atom, *spec, atom.refs(RefKind::Manifest))?,
                content: self.write_ref(atom, *tip, atom.refs(RefKind::Content))?,
                source: self.write_ref(atom, *src, atom.refs(RefKind::Source))?,
            }
        } else {
            AtomReferences {
                manifest: self.write_ref(atom, *tip, atom.refs(RefKind::Manifest))?,
                content: self.write_ref(atom, *tip, atom.refs(RefKind::Content))?,
                source: self.write_ref(atom, *src, atom.refs(RefKind::Source))?,
            }
        })
    }
}

use super::{AtomReferences, AtomTreeIds, GitContent};

impl<'a> AtomReferences<'a> {
    /// Publish atom's to the specified git remote
    ///
    /// Currently the implementation just calls the `git` binary.
    /// Once `gix` is further along we can use it directly.
    pub(super) fn push(self, atom: &'a AtomContext) -> GitContent {
        let remote = atom.git.remote_str.to_owned();
        let mut tasks = atom.git.push_tasks.borrow_mut();

        for r in [&self.content, &self.manifest, &self.source] {
            let r = r.name().as_bstr().to_string();
            let remote = remote.clone();
            let task = async move {
                let result =
                    git::run_git_command(&["push", &remote, format!("{}:{}", r, r).as_str()])?;

                Ok(result)
            };
            tasks.spawn(task);
        }

        GitContent {
            spec: self.manifest.detach(),
            tip: self.content.detach(),
            src: self.source.detach(),
            path: atom.path.to_path_buf(),
            ref_prefix: atom.ref_prefix.clone(),
        }
    }
}

use gix::Object;
/// Helper function to read a blob from an entry
fn read_blob<F, R>(obj: &Object, mut f: F) -> GitResult<R>
where
    F: FnMut(&mut dyn Read) -> io::Result<R>,
{
    let mut reader = obj.data.as_slice();
    Ok(f(&mut reader)?)
}

/// Helper function to create an atom tree from entries
fn atom_tree(entries: &mut Vec<AtomEntry>, atom: &Entry) -> AtomTree {
    entries.push(AtomEntry {
        mode: atom.mode(),
        filename: atom.filename().into(),
        oid: atom.object_id(),
    });

    // git expects tree entries to be sorted
    if entries.len() > 1 {
        entries.sort_unstable();
    }

    AtomTree {
        entries: entries.clone(),
    }
}

impl CommittedAtom {
    pub fn commit(&self) -> &AtomCommit {
        &self.commit
    }
    pub fn tip(&self) -> &ObjectId {
        &self.tip
    }
    pub fn src(&self) -> &ObjectId {
        &self.src
    }
}
