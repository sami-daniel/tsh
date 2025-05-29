mod interpreter;
mod utils;

use std::io::{Write, stdin, stdout};

use anyhow::Result;
use interpreter::executor::execute;

fn main() -> Result<()> {
    let mut stdout = stdout();
    let stdin = stdin();
    let mut buffer = String::new();
    
    loop {
        stdout.write_all(b"$ ")?;
        stdout.flush()?;
        stdin.read_line(&mut buffer)?;
        execute(&buffer, &mut stdout)?;
        stdout.flush()?;
        // TODO: Looks like bad to create a new buffer
        // every iteration of the loop
        buffer = String::new();
    }
}
