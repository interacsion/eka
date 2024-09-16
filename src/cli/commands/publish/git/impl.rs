use super::PublishGitContext;
use crate::cli::logging::LogValue;

use atom::Atom;
use manifest::core::Manifest;

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

struct AtomContext<'a> {
    atom: &'a Atom,
    path: &'a Path,
    context: &'a PublishGitContext<'a>,
    manifest: String,
    source: String,
}

impl<'a> PublishGitContext<'a> {
    /// Method to publish an atom
    pub fn publish_atom(&self, path: &Path) -> Option<()> {
        let dir = path.with_extension("");
        let FoundAtom(atom, atom_entry) = self.find_and_verify_atom(path)?;

        let context = AtomContext::set(&atom, &dir, self);

        let atom_dir_entry = context.maybe_dir();

        let trees = context.write_atom_trees(&atom_entry, atom_dir_entry)?;

        context
            .write_atom_commits(trees)?
            .write_refs(&context)?
            .push(self)
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

        Manifest::get_atom(&content)
            .map_err(|e| {
                tracing::warn!(
                    message = "Ignoring invalid atom manifest",
                    path = %path.display(),
                    commit = %self.commit.id(),
                    error = %e
                );
            })
            .ok()
    }

    /// Compute the ObjectId of the given object without writing it to the repo
    fn compute_hash(&self, obj: &dyn WriteTo) -> Option<ObjectId> {
        use gix::objs;
        use std::io::Cursor;

        let mut buf = Vec::new();
        let mut cursor = Cursor::new(&mut buf);

        obj.write_to(&mut cursor).log_err().ok()?;

        let oid = objs::compute_hash(self.repo.object_hash(), obj.kind(), buf.as_slice());

        Some(oid)
    }

    /// Helper function to write an object to the repository
    fn write_object(&self, obj: impl WriteTo) -> Option<gix::ObjectId> {
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

    fn find_and_verify_atom(&self, path: &Path) -> Option<FoundAtom> {
        let entry = match self.tree_search(path) {
            Some(entry) => entry,
            _ => {
                tracing::warn!(
                    path = %path.display(),
                    commit = %self.commit.id(),
                    "Atom does not exist in the given history"
                );
                return None;
            }
        };

        self.verify_manifest(&entry, path)
            .map(|atom| FoundAtom(atom, entry))
    }
}

enum TreeKind {
    Manifest,
    Source,
}

use std::fmt;

impl fmt::Display for TreeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TreeKind::Manifest => write!(f, "{}", MANIFEST),
            TreeKind::Source => write!(f, "{}", SOURCE),
        }
    }
}

impl<'a> AtomContext<'a> {
    fn set(atom: &'a Atom, path: &'a Path, context: &'a PublishGitContext) -> Self {
        let prefix = format!("atoms/{}/{}", path.to_string_lossy(), atom.version);
        Self {
            atom,
            path,
            context,
            manifest: format!("{}/{}", prefix, TreeKind::Manifest),
            source: format!("{}/{}", prefix, TreeKind::Source),
        }
    }

    fn maybe_dir(&self) -> Option<Entry> {
        match self.context.tree_search(self.path) {
            Some(entry) => entry.mode().is_tree().then_some(entry),
            _ => None,
        }
    }

    fn ref_exists(&self, tree: &AtomTree, kind: TreeKind) -> bool {
        let r = match kind {
            TreeKind::Manifest => &self.manifest,
            TreeKind::Source => &self.source,
        };
        let id = self.context.compute_hash(tree);
        if let Some(id) = id {
            self.context.repo.find_tree(id).is_ok() && self.context.repo.find_reference(r).is_ok()
        } else {
            false
        }
    }

    /// Method to write the atom tree object
    fn write_atom_trees(&self, atom: &Entry, dir: Option<Entry>) -> Option<AtomId> {
        let mut entries: Vec<AtomEntry> = Vec::with_capacity(2);

        let skip = || {
            tracing::info!(path = %self.path.display(), "Atom already exists");
            None
        };

        let manifest_tree = atom_tree(&mut entries, atom);

        let manifest_exists = self.ref_exists(&manifest_tree, TreeKind::Manifest);

        if dir.is_none() && manifest_exists {
            return skip();
        }

        if let Some(entry) = dir {
            let dir_tree = atom_tree(&mut entries, &entry);
            if self.ref_exists(&dir_tree, TreeKind::Source) && manifest_exists {
                return skip();
            }
            let manifest = self.context.write_object(manifest_tree)?;
            let source = Some(self.context.write_object(dir_tree)?);
            AtomId { manifest, source }
        } else {
            let manifest = self.context.write_object(manifest_tree)?;
            AtomId {
                manifest,
                source: None,
            }
        }
        .into()
    }

    /// Method to write atom commits
    fn write_atom_commits(&self, AtomId { manifest, source }: AtomId) -> Option<CommittedAtom> {
        let sig = Signature {
            email: EMPTY.into(),
            name: EMPTY.into(),
            time: gix::date::Time {
                seconds: 0,
                offset: 0,
                sign: gix::date::time::Sign::Plus,
            },
        };
        let commit = AtomCommit {
            tree: manifest,
            parents: Vec::new().into(),
            author: sig.clone(),
            committer: sig,
            encoding: None,
            message: format!("{}: {}", self.atom.id, self.atom.version).into(),
            extra_headers: vec![
                ("origin".into(), self.context.commit.id().as_bytes().into()),
                ("version".into(), FORMAT_VERSION.into()),
            ],
        };
        let id = self.context.write_object(commit.clone())?;
        if let Some(tree) = source {
            let commit = AtomCommit {
                tree,
                parents: vec![id].into(),
                ..commit
            };
            let id = self.context.write_object(commit.clone())?;
            Some(CommittedAtom { commit, id })
        } else {
            Some(CommittedAtom { commit, id })
        }
    }
}

struct FoundAtom<'a>(Atom, Entry<'a>);

/// Struct to hold the result of writing atom commits
#[derive(Debug, Clone)]
struct CommittedAtom {
    commit: AtomCommit,
    id: ObjectId,
}

impl CommittedAtom {
    /// Method to write a single reference to the repository
    fn write_ref<'a>(
        &'a self,
        context: &'a AtomContext,
        id: ObjectId,
        kind: TreeKind,
    ) -> Option<gix::Reference> {
        use gix::refs::transaction::PreviousValue;

        let r = match kind {
            TreeKind::Manifest => &context.manifest,
            TreeKind::Source => &context.source,
        };
        context
            .context
            .repo
            .reference(
                format!("refs/{}", r.to_owned()),
                id,
                PreviousValue::MustNotExist,
                format!(
                    "publish: {}: {}-{}",
                    context.atom.id, context.atom.version, kind
                ),
            )
            .log_err()
            .ok()
    }
    /// Method to write references for the committed atom
    fn write_refs<'a>(&'a self, context: &'a AtomContext) -> Option<AtomReference> {
        let Self { commit, id } = self;

        Some(if let Some(manifest) = commit.parents.first() {
            AtomReference {
                manifest: self.write_ref(context, *manifest, TreeKind::Manifest)?,
                source: self.write_ref(context, *id, TreeKind::Source),
            }
        } else {
            AtomReference {
                manifest: self.write_ref(context, *id, TreeKind::Manifest)?,
                source: None,
            }
        })
    }
}

/// Struct to hold the unique identity of an atom given by the Git object ID of the tree(s) of its contents
struct AtomId {
    manifest: ObjectId,
    source: Option<ObjectId>,
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
    if entries.len() > 1 {
        entries.sort_unstable();
    }

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
