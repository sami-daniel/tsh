use std::{
    io::{Write, stderr, stdin, stdout},
    os::fd::{AsFd, FromRawFd, OwnedFd},
    path::Path,
};

use anyhow::Result;
use nix::{
    fcntl::{OFlag, open},
    sys::stat::Mode,
    unistd::{close, dup, dup2},
};

use crate::{
    interpreter::parser::{Command, RedirectionTarget, RedirectionType},
    utils::report_line_err,
};

use super::resolver::CommandExecutor;

impl Command {
    pub fn exec(self: &Command) -> Result<()> {
        let mut redirect_helper = RedirectHelper::new();
        
        // first step is to configure redirects
        self.configure_redirects(&mut redirect_helper)?;
        
        let command_executor = CommandExecutor::from_command(&self)?;
        let executable = command_executor.builtin_exec;
        let mut stdout = stdout().lock();
        let mut stderr = stderr().lock();

        executable(&stdin(), &mut stdout, &mut stderr)?;
        
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
                                report_line_err();
                            }
                        }
                        RedirectionType::AppendOutput => {
                            if let RedirectionTarget::RealFile(file) = &redirect.target {
                                redirect_helper.redirect_to_file(file, redirect.from_fd, true)?
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

struct RedirectHelper {
    original_fds: Vec<(OwnedFd, OwnedFd)>,
}

impl RedirectHelper {
    fn new() -> Self {
        Self {
            original_fds: vec![],
        }
    }

    fn reset_sources(&mut self) -> Result<()> {
        for (copy, original) in &mut self.original_fds {
            dup2(copy.try_clone()?, original)?;
            close(copy.try_clone()?)?;
        }

        Ok(())
    }

    fn redirect_to_file(&mut self, file: &Path, fd: i32, append: bool) -> Result<()> {
        let flags = OFlag::empty();
        let flags = flags.union(OFlag::O_CREAT);
        let flags = flags.union(OFlag::O_RDWR);
        let flags = if append {
            flags.union(OFlag::O_APPEND)
        } else {
            flags.union(OFlag::O_TRUNC)
        };

        dbg!(flags);

        // Create a duplicated fd for later, we can rollback the file descriptors
        // to its orignal open file descriptors
        // The actual call from libc returns -1 and errno is setted indicating the error. The
        // errors that errno can set are described in: https://www.man7.org/linux/man-pages/man2/open.2.html#ERRORS
        let file_fd = open(file, flags, Mode::from_bits(0o644).unwrap())?;
        // The actual call from libc returns -1 and errno is setted indicating the error. The
        // errors that errno can set are described in: https://www.man7.org/linux/man-pages/man2/dup.2.html#ERRORS
        dup2(file_fd.as_fd(), &mut unsafe {
            // According to docs, this should be open and be owned.
            // Read: https://doc.rust-lang.org/beta/std/io/index.html#io-safety
            let source_fd = OwnedFd::from_raw_fd(fd);
            let dup_fd = dup(source_fd.try_clone()?)?;
            self.original_fds.push((dup_fd, source_fd.try_clone()?));

            source_fd
        })?;
        close(file_fd)?;

        Ok(())
    }
}
