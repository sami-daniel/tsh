use std::{
    io::Write,
    os::fd::{FromRawFd, OwnedFd},
    path::Path,
};

use anyhow::Result;
use nix::{
    fcntl::{OFlag, open},
    sys::stat::Mode,
    unistd::dup2,
};

use crate::{
    interpreter::parser::{Command, RedirectionTarget, RedirectionType},
    utils::report_line_err,
};

impl Command {
    pub fn exec<T>(self: &Command, output_buffer: &mut T) -> Result<()>
    where
        T: Write,
    {
        // first step is to configure redirects
        self.configure_redirects()?;

        Ok(())
    }

    fn configure_redirects(&self) -> Result<()> {
        match self {
            Self::Simple { redirects, .. } => {
                for redirect in redirects {
                    match redirect.kind {
                        RedirectionType::Output => {
                            if let RedirectionTarget::RealFile(file) = &redirect.target {
                                redirect_to_file(file, redirect.from_fd, false)?
                            } else {
                                // Impossible redirect text output (redirect without @) to file descriptor
                                report_line_err();
                            }
                        }
                        RedirectionType::AppendOutput => {
                            if let RedirectionTarget::RealFile(file) = &redirect.target {
                                redirect_to_file(file, redirect.from_fd, true)?
                            } else {
                                // Impossible redirect text output (redirect without @) to file descriptor
                                report_line_err();
                            }
                        }
                        _ => {}
                    }
                }

                Ok(())
            }
        }
    }
}

fn redirect_to_file(file: &Path, fd: i32, append: bool) -> Result<()> {
    let flags = OFlag::empty();
    let flags = flags.union(OFlag::O_CREAT);
    let flags = flags.union(OFlag::O_RDWR);
    let flags = if append {
        flags.union(OFlag::O_APPEND)
    } else {
        flags.union(OFlag::O_TRUNC)
    };

    dbg!(flags);

    // The actual call from libc returns -1 and errno is setted indicating the error. The
    // errors that errno can set are described in: https://www.man7.org/linux/man-pages/man2/open.2.html#ERRORS
    let file_fd = open(file, flags, Mode::from_bits(0o644).unwrap())?;
    // The actual call from libc returns -1 and errno is setted indicating the error. The
    // errors that errno can set are described in: https://www.man7.org/linux/man-pages/man2/dup.2.html#ERRORS
    dup2(file_fd, &mut unsafe { OwnedFd::from_raw_fd(fd) })?;

    Ok(())
}
