use askama::Template;
use std::fmt;

/// Supported shells
#[derive(Debug, Clone, Copy)]
pub enum Shell {
    Bash,
    Elvish,
    Fish,
    Nushell,
    Oil,
    Powershell,
    Xonsh,
    Zsh,
}

impl std::str::FromStr for Shell {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bash" => Ok(Shell::Bash),
            "elvish" => Ok(Shell::Elvish),
            "fish" => Ok(Shell::Fish),
            "nushell" => Ok(Shell::Nushell),
            "oil" => Ok(Shell::Oil),
            "powershell" => Ok(Shell::Powershell),
            "xonsh" => Ok(Shell::Xonsh),
            "zsh" => Ok(Shell::Zsh),
            _ => Err(format!("Unsupported shell: {}", s)),
        }
    }
}

impl fmt::Display for Shell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Shell::Bash => write!(f, "bash"),
            Shell::Elvish => write!(f, "elvish"),
            Shell::Fish => write!(f, "fish"),
            Shell::Nushell => write!(f, "nushell"),
            Shell::Oil => write!(f, "oil"),
            Shell::Powershell => write!(f, "powershell"),
            Shell::Xonsh => write!(f, "xonsh"),
            Shell::Zsh => write!(f, "zsh"),
        }
    }
}

/// Hook mode for tracking worktree changes
#[derive(Debug, Clone, Copy)]
pub enum Hook {
    /// Don't hook into shell - user calls commands manually
    None,
    /// Hook into shell prompt - update tracking on every prompt
    Prompt,
}

impl std::str::FromStr for Hook {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(Hook::None),
            "prompt" => Ok(Hook::Prompt),
            _ => Err(format!("Invalid hook mode: {}", s)),
        }
    }
}

impl fmt::Display for Hook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Hook::None => write!(f, "none"),
            Hook::Prompt => write!(f, "prompt"),
        }
    }
}

/// Shell integration configuration
pub struct ShellInit {
    pub shell: Shell,
    pub cmd_prefix: String,
    pub hook: Hook,
}

impl ShellInit {
    pub fn new(shell: Shell, cmd_prefix: String, hook: Hook) -> Self {
        Self {
            shell,
            cmd_prefix,
            hook,
        }
    }

    /// Generate shell integration code
    pub fn generate(&self) -> Result<String, askama::Error> {
        match self.shell {
            Shell::Bash | Shell::Zsh | Shell::Oil => {
                let template = BashTemplate {
                    shell_name: self.shell.to_string(),
                    cmd_prefix: &self.cmd_prefix,
                    hook: self.hook,
                };
                template.render()
            }
            Shell::Fish => {
                let template = FishTemplate {
                    cmd_prefix: &self.cmd_prefix,
                    hook: self.hook,
                };
                template.render()
            }
            Shell::Nushell => {
                let template = NushellTemplate {
                    cmd_prefix: &self.cmd_prefix,
                    hook: self.hook,
                };
                template.render()
            }
            Shell::Powershell => {
                let template = PowershellTemplate {
                    cmd_prefix: &self.cmd_prefix,
                    hook: self.hook,
                };
                template.render()
            }
            Shell::Elvish => {
                let template = ElvishTemplate {
                    cmd_prefix: &self.cmd_prefix,
                    hook: self.hook,
                };
                template.render()
            }
            Shell::Xonsh => {
                let template = XonshTemplate {
                    cmd_prefix: &self.cmd_prefix,
                    hook: self.hook,
                };
                template.render()
            }
        }
    }
}

/// Bash/Zsh shell template
#[derive(Template)]
#[template(path = "bash.sh", escape = "none")]
struct BashTemplate<'a> {
    shell_name: String,
    cmd_prefix: &'a str,
    hook: Hook,
}

/// Fish shell template
#[derive(Template)]
#[template(path = "fish.fish", escape = "none")]
struct FishTemplate<'a> {
    cmd_prefix: &'a str,
    hook: Hook,
}

/// Nushell shell template
#[derive(Template)]
#[template(path = "nushell.nu", escape = "none")]
struct NushellTemplate<'a> {
    cmd_prefix: &'a str,
    hook: Hook,
}

/// PowerShell template
#[derive(Template)]
#[template(path = "powershell.ps1", escape = "none")]
struct PowershellTemplate<'a> {
    cmd_prefix: &'a str,
    hook: Hook,
}

/// Elvish shell template
#[derive(Template)]
#[template(path = "elvish.elv", escape = "none")]
struct ElvishTemplate<'a> {
    cmd_prefix: &'a str,
    hook: Hook,
}

/// Xonsh shell template
#[derive(Template)]
#[template(path = "xonsh.xsh", escape = "none")]
struct XonshTemplate<'a> {
    cmd_prefix: &'a str,
    hook: Hook,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_from_str() {
        assert!(matches!("bash".parse::<Shell>(), Ok(Shell::Bash)));
        assert!(matches!("BASH".parse::<Shell>(), Ok(Shell::Bash)));
        assert!(matches!("fish".parse::<Shell>(), Ok(Shell::Fish)));
        assert!(matches!("zsh".parse::<Shell>(), Ok(Shell::Zsh)));
        assert!("invalid".parse::<Shell>().is_err());
    }

    #[test]
    fn test_hook_from_str() {
        assert!(matches!("none".parse::<Hook>(), Ok(Hook::None)));
        assert!(matches!("prompt".parse::<Hook>(), Ok(Hook::Prompt)));
        assert!("invalid".parse::<Hook>().is_err());
    }
}
