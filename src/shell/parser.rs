use std::{
    path::{Path, PathBuf},
    vec,
};

use anyhow::{Result, anyhow};
use regex::Regex;

#[derive(Debug)]
pub enum Command<'a> {
    Simple {
        command_main: &'a str,
        args: Box<&'a [&'a str]>,
        redirects: Box<&'a [&'a str]>,
    },
}

#[derive(Debug)]
pub enum RedirectionType {
    Output,
    AppendOutput,
    RedirectToFileDescriptor,
}

#[derive(Debug)]
pub enum RedirectionTarget {
    RealFile(PathBuf),
    FileDescriptor(i32),
}

#[derive(Debug)]
pub struct Redirect {
    pub from_fd: i32,
    pub kind: RedirectionType,
    pub target: RedirectionTarget,
}

pub fn try_parse_input(input: &str) -> Result<Command> {
    let tokens = shell_words::split(input)?;
    let tokens: Vec<&str> = tokens.iter().map(|part| part.as_str()).collect();
    let mut iter = tokens.iter();

    let main_command = *if let Some(command) = iter.next() {
        command
    } else {
        return Err(anyhow!("Invalid command: \"\""));
    };

    while let Some(payload) = iter.next() {

    };

    todo!()
}

fn parse_redirects(input: &str) -> Result<Vec<Redirect>> {
    todo!()
}
