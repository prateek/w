use std::error::Error;
use vergen_gitcl::{Emitter, GitclBuilder as GitBuilder};

pub fn main() -> Result<(), Box<dyn Error>> {
    let git = GitBuilder::default().describe(true, true, None).build()?;
    Emitter::default().add_instructions(&git)?.emit()?;
    Ok(())
}
