mod interpreter;
mod utils;

use std::io::Write;

use anyhow::Result;
use interpreter::executor;

use crate::utils::{POISONED_LOCK_MSG_ERR, STDIN, STDOUT};

fn main() -> Result<()> {
    let mut buffer = String::new();

    loop {
        {
            let stdout = STDOUT.lock().expect(POISONED_LOCK_MSG_ERR);
            let stdin = STDIN.lock().expect(POISONED_LOCK_MSG_ERR);
            let stdin = stdin.borrow_mut();
            let mut stdout = stdout.borrow_mut();

            stdout.write_all(b"$ ")?;
            stdout.flush()?;
            stdin.read_line(&mut buffer)?;
        }

        if let Err(e) = executor::execute(&buffer) {
            eprintln!("{}", e)
        }

        buffer.clear();
    }
}
