mod engine;
mod resolver;

use super::parser::try_parse_input;
use anyhow::Result;

pub fn execute(input: &str) -> Result<()> {
    if let Some(command) = try_parse_input(input)? {
        dbg!(&command);

        command.exec()?;
    }

    Ok(())
}
