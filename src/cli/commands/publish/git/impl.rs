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

use super::PublishGitContext;

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
            .write_refs(self.repo, &atom, &no_ext)?
            .push(self)
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
                let commit = self
                    .commit
                    .short_id()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|_| self.commit.id().to_string());

                tracing::warn!(
                    message = "Ignoring invalid atom manifest",
                    path = %path.display(),
                    commit = %commit,
                    error = %format!("{}", e)
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
                format!(
                    "refs/atoms/{}/{}/{}",
                    ref_path.display(),
                    atom.version,
                    kind
                ),
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
pub(super) struct AtomReference<'a> {
    manifest: Reference<'a>,
    source: Option<Reference<'a>>,
}

impl<'a> AtomReference<'a> {
    /// Publish atom's to the specified git remote
    ///
    /// Currently the implementation just calls the `git` binary.
    /// Once `gix` is further along we can use it directly.
    fn push(&'a self, context: &'a PublishGitContext) -> Option<()> {
        let remote = context.remote.name()?.as_symbol()?.to_owned();
        let manifest_ref = self.manifest.name().as_bstr().to_string();
        let source_ref = self.source.as_ref().map(|r| r.name().as_bstr().to_string());
        let mut tasks = context.push_tasks.borrow_mut();

        if let Some(r) = source_ref {
            let remote = remote.clone();
            let task = async move {
                let result = run_git_command(&["push", &remote, format!("{}:{}", r, r).as_str()])?;

                Ok(result)
            };
            tasks.spawn(task);
        }

        let task = async move {
            let result = run_git_command(&[
                "push",
                &remote,
                format!("{}:{}", manifest_ref, manifest_ref).as_str(),
            ])?;
            Ok(result)
        };
        tasks.spawn(task);

        // TODO: figure out what is broken here for native pushing, or wait for upstream support
        //
        // use gix::diff::object::Data;
        // use gix::index::hash::Kind as HashKind;
        // use gix::remote::Direction;
        // use gix::worktree::object::Kind;
        // use gix_pack::data::output::{
        //     bytes::FromEntriesIter, count::PackLocation, Count, Entry as PackEntry,
        // };
        // use gix_pack::data::Version;
        // use gix_transport::{
        //     client::{MessageKind, WriteMode},
        //     Service,
        // };
        // use std::io::Cursor;

        // let id = if let Some(r) = self.source.clone() {
        //     r.clone().peel_to_id_in_place()
        // } else {
        //     self.manifest.peel_to_id_in_place()
        // }
        // .log_err()?
        // .detach();

        // let c = Count {
        //     id,
        //     entry_pack_location: PackLocation::NotLookedUp,
        // };

        // let d = Data::new(Kind::Commit, id.deref().as_bytes());
        // let entry = PackEntry::from_data(&c, &d).map(|x| vec![x]);

        // // 1. Create the pack file
        // let entries_iter = std::iter::once(entry);
        // let mut output = Cursor::new(Vec::new());
        // FromEntriesIter::new(
        //     entries_iter,
        //     &mut output,
        //     5, // number of entries (just one commit)
        //     Version::V2,
        //     HashKind::Sha1,
        // )
        // .next()
        // .and_then(|x| x.log_err().ok());
        // let pack_data = output.into_inner();

        // let mut client_req = context.remote.connect(Direction::Push).log_err()?;
        // let url = context.remote.url(Direction::Push);
        // if let Some(url) = url {
        //     let creds = client_req.configured_credentials(url.clone());
        //     if let Ok(creds) = creds {
        //         client_req = client_req.with_credentials(creds);
        //     }
        // };
        // let client = client_req.transport_mut();
        // client.handshake(Service::ReceivePack, &[]).log_err()?;
        // let mut writer = client
        //     .request(WriteMode::Binary, MessageKind::Flush, false)
        //     .log_err()?;
        // writer.write_all(&pack_data).log_err()?;
        // writer.flush().log_err()?;

        Some(())
    }
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

    // git expects tree entries to be sorted
    entries.sort_unstable();

    AtomTree {
        entries: entries.clone(),
    }
}

fn run_git_command(args: &[&str]) -> io::Result<Vec<u8>> {
    use std::process::Command;
    let output = Command::new("git").args(args).output()?;

    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            String::from_utf8_lossy(&output.stderr),
        ))
    }
}
const FORMAT_VERSION: &str = "1";
const EMPTY: &str = "";
const SOURCE: &str = "source";
const MANIFEST: &str = "manifest";
