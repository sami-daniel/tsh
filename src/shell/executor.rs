use anyhow::Result;

use super::parser::try_parse_input;

pub fn execute(input: &String) -> Result<()> {
    let command = try_parse_input(input.as_str())?;
    Ok(())
}