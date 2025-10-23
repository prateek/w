//! Directive output mode for shell integration

use anstream::adapter::strip_str;
use std::io::{self, Write};
use std::path::Path;

/// Directive output mode for shell integration
///
/// Outputs NUL-terminated directives for shell wrapper to parse and execute.
pub struct DirectiveOutput;

impl DirectiveOutput {
    pub fn new() -> Self {
        Self
    }

    pub fn success(&mut self, message: String) -> io::Result<()> {
        let plain = strip_str(&message).to_string();
        write!(io::stdout(), "{}\0", plain)
    }

    pub fn change_directory(&mut self, path: &Path) -> io::Result<()> {
        write!(io::stdout(), "__WORKTRUNK_CD__{}\0", path.display())
    }

    pub fn execute(&mut self, command: String) -> io::Result<()> {
        write!(io::stdout(), "__WORKTRUNK_EXEC__{}\0", command)
    }

    pub fn flush(&mut self) -> io::Result<()> {
        writeln!(io::stdout())?;
        io::stdout().flush()
    }

    pub fn is_interactive(&self) -> bool {
        false
    }
}

impl Default for DirectiveOutput {
    fn default() -> Self {
        Self::new()
    }
}
