use std::{
    cell::RefCell,
    env,
    io::{Stderr, Stdin, Stdout, stderr, stdin, stdout},
    path::PathBuf,
    sync::Mutex,
};

use anyhow::Result;
use lazy_static::lazy_static;

pub const POISONED_LOCK_MSG_ERR: &str = "Poisoned lock found";

lazy_static! {
    pub static ref STDOUT: Mutex<RefCell<Stdout>> = Mutex::new(RefCell::new(stdout()));
    pub static ref STDERR: Mutex<RefCell<Stderr>> = Mutex::new(RefCell::new(stderr()));
    pub static ref STDIN: Mutex<RefCell<Stdin>> = Mutex::new(RefCell::new(stdin()));
}

#[inline(always)]
pub fn report_line_err(args: Option<&str>) {
    eprintln!("Reported error at {}", line!());
    if let Some(args) = args {
        eprintln!("Args: {args}");
    }
}

#[inline]
pub fn get_cwd() -> Result<PathBuf> {
    Ok(env::current_dir()?)
}
