mod engine;
mod resolver;

use std::io::Write;

use super::parser::try_parse_input;
use anyhow::Result;

pub fn execute<T>(input: &str, output_buffer: &mut T) -> Result<()>
where
    T: Write,
{
    if let Some(command) = try_parse_input(input)? {
        dbg!(&command);

        command.exec()?;
    }

    Ok(())
}
