use crate::{
    interpreter::parser::Command,
    utils::{POISONED_LOCK_MSG_ERR, STDOUT},
};
use anyhow::{Result, anyhow};
use std::io::{Read, Write};

pub struct CommandExecutor {
    pub target: ExecutorTarget,
    pub executable: Box<dyn FnOnce() -> Result<()>>,
}

pub enum ExecutorTarget {
    Bultin(BuiltinTarget),
}

pub enum BuiltinTarget {
    Echo,
}

impl CommandExecutor {
    pub fn from_command(command: &Command) -> Result<Self> {
        match command {
            Command::Simple {
                command_main, args, ..
            } => {
                let cmd_name = &command_main[..];
                match cmd_name {
                    "echo" => Ok(Self {
                        target: ExecutorTarget::Bultin(BuiltinTarget::Echo),
                        executable: build_echo_exec(args),
                    }),
                    _ => Err(anyhow!("Command not recognized: {}", cmd_name)),
                }
            }
        }
    }
}

fn build_echo_exec(args: &Vec<String>) -> Box<dyn FnOnce() -> Result<()>> {
    // TODO: Refactor to not clone args.
    let args = args.clone();

    Box::new(move || {
        let stdout = STDOUT.lock().expect(&POISONED_LOCK_MSG_ERR);
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
