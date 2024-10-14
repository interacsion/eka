use std::path::{Path, PathBuf};
use std::process::{Command as UnsafeCommand, ExitCode};
use std::{env, fs};

use birdcage::process::Command;
use birdcage::{Birdcage, Exception, Sandbox};
use thiserror::Error;

#[derive(Error, Debug)]
enum NixecError {
    #[error("No `nix` executable in PATH")]
    NoNix,
    #[error(transparent)]
    ExceptionFailed(#[from] birdcage::error::Error),
    #[error(transparent)]
    CommandFailed(#[from] std::io::Error),
    #[error("Failed to determine nix store path")]
    StorePath,
}

type Result<T> = std::result::Result<T, NixecError>;

fn main() -> Result<ExitCode> {
    let nix_dir = bin_dir("nix")?;
    let nix_instantiate = nix_dir.join("nix-instantiate");
    let cwd = env::current_dir()?;

    let args: Vec<String> = env::args().collect();
    let sandbox_args = &args[1..];

    let mut sandbox = Birdcage::new();

    sandbox.add_exception(Exception::Read(cwd))?;

    let nix_store: PathBuf = String::from_utf8(
        UnsafeCommand::new(nix_instantiate.clone())
            .args(["--eval", "--expr", "builtins.storeDir"])
            .output()?
            .stdout,
    )
    .map_err(|_| NixecError::StorePath)?
    .trim()
    .trim_matches('"')
    .into();

    sandbox.add_exception(Exception::ExecuteAndRead(
        nix_store
            .parent()
            .map(Path::to_path_buf)
            .ok_or(NixecError::StorePath)?,
    ))?;
    unsafe { env::set_var("HOME", "/homeless-shelter") };
    sandbox.add_exception(Exception::Environment("HOME".into()))?;

    sandbox.add_exception(Exception::ExecuteAndRead(nix_dir))?;
    let mut command = Command::new(nix_instantiate);
    command.args(sandbox_args);

    let output = sandbox.spawn(command)?.wait_with_output()?;

    Ok(ExitCode::from(output.status.code().unwrap_or(1) as u8))
}

fn bin_dir(exec_name: &str) -> Result<PathBuf> {
    env::var_os("PATH")
        .and_then(|paths| {
            env::split_paths(&paths)
                .filter_map(|dir| {
                    let full_path = dir.join(exec_name);
                    if full_path.is_file() {
                        fs::canonicalize(full_path)
                            .ok()
                            .and_then(|p| p.parent().map(Path::to_path_buf))
                    } else {
                        None
                    }
                })
                .next()
        })
        .ok_or(NixecError::NoNix)
}
