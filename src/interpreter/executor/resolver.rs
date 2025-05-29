use crate::interpreter::parser::Command;
use anyhow::{Result, anyhow};
use std::io::{Read, Write};

pub struct CommandExecutor {
    pub target: ExecutorTarget,
    pub builtin_exec: Box<dyn FnOnce(&dyn Read, &mut dyn Write, &mut dyn Write) -> Result<()>>,
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
                        builtin_exec: build_echo_exec(args),
                    }),
                    _ => Err(anyhow!("Command not recognized: {}", cmd_name)),
                }
            }
        }
    }
}

fn build_echo_exec(
    args: &Vec<String>,
) -> Box<dyn FnOnce(&dyn Read, &mut dyn Write, &mut dyn Write) -> Result<()>> {
    let args = args.clone();
    Box::new(move |_, stdout, _| {
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
