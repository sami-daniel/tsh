use std::{os::fd::IntoRawFd, path::Path};

use anyhow::Result;
use nix::{
    errno::Errno,
    fcntl::{OFlag, open},
    libc,
    sys::stat::Mode,
};

use crate::{
    interpreter::parser::{Command, RedirectionTarget, RedirectionType},
    utils::report_line_err,
};

use super::resolver::from_command;

impl Command {
    pub fn exec(self: &Command) -> Result<()> {
        let mut redirect_helper = RedirectHelper::new();
        self.configure_redirects(&mut redirect_helper)?;
        let executable = from_command(self)?;

        executable()?;

        redirect_helper.reset_sources()?;

        Ok(())
    }

    fn configure_redirects(&self, redirect_helper: &mut RedirectHelper) -> Result<()> {
        match self {
            Self::Simple { redirects, .. } => {
                for redirect in redirects {
                    match redirect.kind {
                        RedirectionType::Output => {
                            if let RedirectionTarget::RealFile(file) = &redirect.target {
                                redirect_helper.redirect_to_file(file, redirect.from_fd, false)?
                            } else {
                                // Impossible redirect text output (redirect without @) to file descriptor
                                report_line_err(Some(
                                    "Fatal TSH Error: File redirection to file descriptor detected",
                                ));
                            }
                        }
                        RedirectionType::AppendOutput => {
                            if let RedirectionTarget::RealFile(file) = &redirect.target {
                                redirect_helper.redirect_to_file(file, redirect.from_fd, true)?
                            } else {
                                // Impossible redirect text output (redirect without @) to file descriptor
                                report_line_err(Some(
                                    "Fatal TSH Error: File redirection to file descriptor detected",
                                ));
                            }
                        }
                        _ => {
                            if let RedirectionTarget::FileDescriptor(fd) = &redirect.target {
                                redirect_helper.redirect_to_fd(redirect.from_fd, *fd)?
                            } else {
                                // Impossible redirect file descriptor (redirect with @) to real file
                                report_line_err(Some(
                                    "Fatal TSH Error: File descriptor redirection to File detected",
                                ));
                            }
                        }
                    }
                }

                Ok(())
            }
        }
    }
}

struct RedirectHelper {
    original_fds: Vec<(i32, i32)>,
}

impl RedirectHelper {
    fn new() -> Self {
        Self {
            original_fds: vec![],
        }
    }

    fn has_original_for(&self, fd: i32) -> bool {
        self.original_fds
            .iter()
            .any(|(_, original)| original == &fd)
    }

    fn reset_sources(&mut self) -> Result<()> {
        for (dup, original) in self.original_fds.iter() {
            // SAFETY:
            // If the duplication returned some error and setted errno, we catch and return it. If
            // both are the same (like source_fd are the same fd of file), we don't do anything. It
            // can be dangerous if used incorrectly, like, if dup refers to other file descriptor.
            let result = unsafe { libc::dup2(*dup, *original) };
            if result == -1 {
                return Err(Errno::last().into());
            }

            // SAFETY:
            // Assuming that the self.original_fds are the correctly fds (copies of original)
            // we can safely close the dup2, cause now we are 'poiting' to the fd to the original
            // open file description.
            let result = unsafe { libc::close(*dup) };
            if result == -1 {
                return Err(Errno::last().into());
            }
        }

        Ok(())
    }

    fn redirect_to_file(&mut self, file: &Path, fd: i32, append: bool) -> Result<()> {
        if !self.has_original_for(fd) {
            // Create a duplicated fd for later, we can rollback the file descriptors
            // to its orignal open file descriptors

            // SAFETY:
            // If the duplication returned some error and setted errno, we catch it and proceed.
            let result = unsafe { libc::dup(fd) };
            if result == -1 {
                return Err(Errno::last().into());
            }
            self.original_fds.push((result, fd));
        }

        let flags = OFlag::O_CREAT
            | OFlag::O_RDWR
            | if append {
                OFlag::O_APPEND
            } else {
                OFlag::O_TRUNC
            };

        // The actual call from libc returns -1 and errno is setted indicating the error. The
        // errors that errno can set are described in: https://www.man7.org/linux/man-pages/man2/open.2.html#ERRORS
        let file_fd: i32 = open(file, flags, Mode::from_bits(0o644).unwrap())?.into_raw_fd();

        // SAFETY:
        // If the duplication returned some error and setted errno, we catch and return it. If
        // both are the same (like source_fd are the same fd of file), we don't do anything.
        let result = unsafe { libc::dup2(file_fd, fd) };
        if result == -1 {
            return Err(Errno::last().into());
        } else if result == fd {
            return Ok(());
        }

        // SAFETY:
        // If the duplication returned some error and setted errno, we catch it and proceed.
        // Here we can safelly close the file_descriptor of file cause now we have the 'fd'
        // new_fd 'pointing' to the file_fd open file description, so even closign it, the
        // kernel will mantain the open file description open, cause we still have the 'fd'
        // 'pointing' to it.
        let result = unsafe { libc::close(file_fd) };
        if result == -1 {
            return Err(Errno::last().into());
        }

        Ok(())
    }

    fn redirect_to_fd(&mut self, source: i32, dest: i32) -> Result<()> {
        if !self.has_original_for(source) {
            // Create a duplicated fd for later, we can rollback the file descriptors
            // to its orignal open file descriptors

            // SAFETY:
            // If the duplication returned some error and setted errno, we catch it and proceed.
            let result = unsafe { libc::dup(source) };
            if result == -1 {
                return Err(Errno::last().into());
            }
            self.original_fds.push((result, source));
        }

        // SAFETY:
        // If the duplication returned some error and setted errno, we catch and return it. If
        // both are the same (like source_fd are the same fd of open file description), we don't do anything.
        let result = unsafe { libc::dup2(dest, source) };
        if result == -1 {
            return Err(Errno::last().into());
        } else if result == source {
            return Ok(());
        }

        // SAFETY:
        // If the duplication returned some error and setted errno, we catch it and proceed.
        // Here we can safelly close the file_descriptor of file cause now we have the 'fd'
        // new_fd 'pointing' to the file_fd open file description, so even closign it, the
        // kernel will mantain the open file description open, cause we still have the 'fd'
        // 'pointing' to it.
        let result = unsafe { libc::close(dest) };
        if result == -1 {
            return Err(Errno::last().into());
        }

        Ok(())
    }
}
