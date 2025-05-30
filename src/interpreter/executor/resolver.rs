use crate::{
    interpreter::parser::Command,
    utils::{EXECUTABLES, POISONED_LOCK_MSG_ERR, STDERR, STDOUT, get_cwd, get_executable_path},
};
use anyhow::{Error, Result};
use nix::{
    libc,
    sys::wait::waitpid,
    unistd::{ForkResult, execve, fork},
};
use std::{
    ffi::{CStr, CString},
    io::Write,
    os::unix::ffi::OsStrExt,
    process::exit,
    thread,
};

pub struct CommandExecutor {
    pub target_type: TargetExecutor,
    pub executable: Box<dyn FnOnce() -> Result<()> + 'static + Send>,
}

pub enum TargetExecutor {
    Builtin,
    Ext,
}

pub fn from_command(command: &Command) -> Result<CommandExecutor> {
    #[allow(unused_assignments)] // this is being used, but is saying that isn't
    let mut job = false;
    
    let mut executor = match command {
        Command::Simple {
            command_name,
            args,
            dont_wait,
            ..
        } => {
            job = *dont_wait;
            let cmd_name = &str::to_lowercase(command_name)[..];
            match cmd_name {
                "echo" => Ok::<CommandExecutor, Error>(CommandExecutor {
                    target_type: TargetExecutor::Builtin,
                    executable: build_echo_exec(args),
                }),
                "exit" => Ok(CommandExecutor {
                    target_type: TargetExecutor::Builtin,
                    executable: build_exit_exec(args),
                }),
                "pwd" => Ok(CommandExecutor {
                    target_type: TargetExecutor::Builtin,
                    executable: build_pwd_exec(),
                }),
                _ => Ok(CommandExecutor {
                    target_type: TargetExecutor::Ext,
                    executable: build_ext_exec(command_name, args, *dont_wait),
                }),
            }
        }
    }?;

    match executor.target_type {
        TargetExecutor::Builtin => {
            fn build_builtin_exec_jobbed_from_original<E>(
                e: E,
            ) -> Box<dyn FnOnce() -> Result<()> + 'static + Send>
            where
                E: FnOnce() -> Result<()> + 'static + Send,
            {
                Box::new(move || {
                    thread::spawn(move || {
                        let _ = e();
                    });
                    Ok(())
                })
            }

            if job {
                executor.executable = build_builtin_exec_jobbed_from_original(executor.executable);
            }

            Ok(executor)
        }
        _ => Ok(executor),
    }
}

#[inline(always)]
fn build_echo_exec(args: &[String]) -> Box<dyn FnOnce() -> Result<()> + Send> {
    // TODO: Refactor to not clone args.
    let args = args.to_owned();
    Box::new(move || {
        let stdout = STDOUT.lock().expect(POISONED_LOCK_MSG_ERR);
        let mut stdout = stdout.borrow_mut();

        let mut peekable = args.iter().peekable();
        while let Some(arg) = peekable.next() {
            stdout.write_all(arg.as_bytes())?;
            if peekable.peek().is_some() {
                stdout.write_all(b" ")?;
            }
        }
        stdout.write_all(b"\n")?;
        stdout.flush()?;

        Ok(())
    })
}

#[inline(always)]
fn build_exit_exec(args: &[String]) -> Box<dyn FnOnce() -> Result<()> + Send> {
    // TODO: Refactor to not clone args.
    let args = args.to_owned();
    Box::new(move || {
        if let Some(exit_code) = args.first() {
            if let Ok(exit_code) = exit_code.parse::<i32>() {
                exit(exit_code)
            } else {
                exit(0)
            }
        } else {
            exit(0)
        }
    })
}

#[inline(always)]
fn build_pwd_exec() -> Box<dyn FnOnce() -> Result<()> + Send> {
    Box::new(|| -> Result<()> {
        let stdout = STDOUT.lock().expect(POISONED_LOCK_MSG_ERR);
        let mut stdout = stdout.borrow_mut();
        stdout.write_all(get_cwd()?.as_os_str().as_bytes())?;
        stdout.write_all(b"\n")?;

        Ok(())
    })
}

#[inline(always)]
fn build_ext_exec(
    command_name: &str,
    args: &[String],
    job: bool,
) -> Box<dyn FnOnce() -> Result<()> + Send> {
    // TODO: Refactor to not clone args.
    let command_name = command_name.to_owned();
    let args = args.to_owned();
    Box::new(move || {
        let executables = EXECUTABLES.lock().expect(POISONED_LOCK_MSG_ERR);
        let executables = executables.borrow();
        let executable_path = get_executable_path(&command_name[..], &executables);
        let stderr = STDERR.lock().expect(POISONED_LOCK_MSG_ERR);
        let mut stderr = stderr.borrow_mut();

        if let Some(path) = executable_path {
            let c_path = CString::new(
                path.as_os_str()
                    .to_str()
                    .expect("Impossible to fail cause the Path is valid utf8 str")
                    .as_bytes(),
            )
            .expect("Impossible to have \\0 in Rust string");
            let mut args = args
                .iter()
                .map(|s| CString::new(s.as_bytes()).expect("Impossible have \\0 in Rust string"))
                .collect::<Vec<_>>();
            args.insert(0, c_path.clone());
            let args = args.iter().map(|s| s.as_c_str()).collect::<Vec<_>>();
            let env: Vec<&CStr> = vec![];

            // SAFETY:
            // We are immediatly invoking execve after fork, so no 'abandoned locks'
            // or unreleasead mutexes can not be touched by Rust and consequently no
            // deadlocks can happen.
            let fork = unsafe { fork()? };
            match fork {
                ForkResult::Child => {
                    execve(c_path.as_c_str(), &args, &env)?;

                    // SAFETY:
                    // If we touch here, means that execve call not work and didnt replaced
                    // the current process image. We can no longer execute absolute any Rust
                    // code on this child process, cause we may try to touch an resource that
                    // are with an 'abandoned lock' or unreleased mutexes protecting it and
                    // that would cause a deadlock and the child process will never terminate.
                    // So we can not use std::process:;exit, cause it may touch some of the things
                    // described above, executing variable Drop, etc. So the libc::exit are
                    // more safe, in this case, than std::process::exit.
                    unsafe { libc::exit(1) };
                }
                ForkResult::Parent { child, .. } => {
                    if !job {
                        waitpid(child, None)?;
                    }
                }
            }
        } else {
            stderr.write_all(format!("Command not found: {}\n", command_name).as_bytes())?;
        }

        Ok(())
    })
}
