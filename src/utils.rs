use std::{env, path::PathBuf};

use anyhow::Result;

#[inline(always)]
pub fn report_line_err() {
    eprintln!("{}", line!());
}

#[inline]
pub fn get_cwd() -> Result<PathBuf> {
    Ok(env::current_dir()?)
}
