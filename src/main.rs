mod interpreter;
mod utils;

use std::{io::Write, thread};

use anyhow::Result;
use interpreter::executor;

use crate::utils::{EXECUTABLES, POISONED_LOCK_MSG_ERR, STDIN, STDOUT, get_executables_in_path};

fn main() -> Result<()> {
    let mut buffer = String::new();

    loop {
        thread::spawn(|| {
            // FIXME: This seems a little bad. I guess it should be replaced to an mechanism of communication
            // with aptd (or the daemon of the current package manager of the system). Only if we have more
            // packages installed, we execute this, otherwise no.
            let executables = EXECUTABLES.lock().expect(POISONED_LOCK_MSG_ERR);
            let mut executables = executables.borrow_mut();

            executables.clear();
            executables.extend(get_executables_in_path());
        });

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
