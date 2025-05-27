use std::{io::{stdin, stdout, Write}};

use anyhow::Result;
use shell::executor::execute;

mod shell;

fn main() -> Result<()> {
    let mut stdout = stdout();
    let stdin = stdin();
    let mut buffer = String::new();

    stdout.write_all(b"$ ")?;
    stdout.flush()?;

    loop {
        stdout.write_all(b"$ ")?;
        stdin.read_line(&mut buffer)?;
        stdout.flush()?;
        execute(&buffer)?;
        // TODO: Looks like bad to create a new buffer
        // every iteration of the loop
        buffer = String::new();
    }
}
