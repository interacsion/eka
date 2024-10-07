use super::*;
use tempfile::TempDir;

fn init_repo_and_remote() -> Result<(TempDir, TempDir), anyhow::Error> {
    let repo_dir = tempfile::tempdir()?;
    let remote_dir = tempfile::tempdir()?;
    let repo = gix::init(repo_dir.as_ref())?;
    let remote = gix::init_bare(remote_dir.as_ref())?;
    let no_parents: Vec<gix::ObjectId> = vec![];
    remote.commit(
        "refs/heads/master",
        "init",
        repo.empty_tree().id(),
        no_parents.clone(),
    )?;
    run_git_command(&[
        "-C",
        repo_dir.as_ref().to_string_lossy().as_ref(),
        "remote",
        "add",
        "origin",
        format!("file://{}", remote.git_dir().display()).as_str(),
    ])?;
    Ok((repo_dir, remote_dir))
}

#[test]
fn init_repo() -> Result<(), anyhow::Error> {
    let (dir, _remote) = init_repo_and_remote()?;
    let repo = gix::open(dir.as_ref())?;
    repo.ekala_init("origin".into())?;
    assert!(repo.is_ekala_store("origin"));
    Ok(())
}
