use std::{
    cell::RefCell,
    env,
    fs::File,
    io::{Stderr, Stdin, Stdout, stderr, stdin, stdout},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};

use anyhow::Result;
use lazy_static::lazy_static;

pub const POISONED_LOCK_MSG_ERR: &str = "Poisoned lock found";

lazy_static! {
    pub static ref STDOUT: Mutex<RefCell<Stdout>> = Mutex::new(RefCell::new(stdout()));
    pub static ref STDERR: Mutex<RefCell<Stderr>> = Mutex::new(RefCell::new(stderr()));
    pub static ref STDIN: Mutex<RefCell<Stdin>> = Mutex::new(RefCell::new(stdin()));
    pub static ref EXECUTABLES: Mutex<RefCell<Vec<PathBuf>>> =
        Mutex::new(RefCell::new(get_executables_in_path()));
}

pub fn report_line_err(args: Option<&str>) {
    eprintln!("Reported error at {}", line!());
    if let Some(args) = args {
        eprint!("Args: {args}");
    }
}

#[inline(always)]
pub fn get_cwd() -> Result<PathBuf> {
    Ok(env::current_dir()?)
}

#[inline(always)]
pub fn get_env(key: &str) -> Result<String> {
    Ok(env::var(key)?)
}

pub fn get_executable_path<'a>(
    executable_name: &str,
    executables: &'a [PathBuf],
) -> Option<&'a Path> {
    // We cannot directly use EXECUTABLES cause we don't an way
    // to specify lifetimes to it, but with an argument list we have

    for executable in executables.iter() {
        if let Some(executable_file_name) = executable.file_name() {
            if executable_file_name == executable_name {
                return Some(executable);
            }
        }
    }

    None
}

pub fn get_executables_in_path() -> Vec<PathBuf> {
    let executables = Arc::new(Mutex::new(vec![]));

    if let Ok(path) = get_env("PATH") {
        for dir in env::split_paths(&path) {
            let executables = Arc::clone(&executables);
            thread::spawn(move || {
                // maybe the path in PATH variable does not exist
                if let Ok(dir_entries) = std::fs::read_dir(Path::new(&dir)) {
                    for entry in dir_entries.flatten() {
                        // according to docs, if we have an IO error during
                        // the iteration, the item will return Error
                        if is_executable(entry.path().as_path()).is_ok() {
                            let mut vec = executables.lock().expect(POISONED_LOCK_MSG_ERR);
                            vec.push(entry.path());
                        }
                    }
                }
            });
        }
    }

    let executables = executables.lock().expect(POISONED_LOCK_MSG_ERR);
    executables.clone()
}

fn is_executable(path: &Path) -> Result<bool> {
    Ok(File::open(path)?.metadata()?.permissions().mode() & 0o111 != 0)
}
