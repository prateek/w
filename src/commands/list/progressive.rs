/// Rendering mode for list command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    /// Buffered: collect all data, then render (traditional)
    Buffered,
    /// Progressive: show rows immediately, update as data arrives
    Progressive,
}

impl RenderMode {
    /// Determine rendering mode based on CLI flags and TTY status
    ///
    /// # Arguments
    ///
    /// * `progressive` - Rendering mode (Some(true) = --progressive, Some(false) = --no-progressive, None = auto)
    /// * `directive_mode` - True if in directive mode (--internal), affects which stream to check
    ///
    /// In directive mode, table output goes to stderr, so we check stderr's TTY status.
    /// In interactive mode, table output goes to stdout, so we check stdout's TTY status.
    pub fn detect(progressive: Option<bool>, directive_mode: bool) -> Self {
        // Priority 1: Explicit CLI flag
        match progressive {
            Some(true) => return RenderMode::Progressive,
            Some(false) => return RenderMode::Buffered,
            None => {} // Fall through to auto-detection
        }

        // Priority 2: Auto-detect based on TTY
        // Check the appropriate stream based on output mode:
        // - Directive mode: check stderr (where table output goes via output::raw())
        // - Interactive mode: check stdout (where table output goes)
        use std::io::IsTerminal;
        let is_tty = if directive_mode {
            std::io::stderr().is_terminal()
        } else {
            std::io::stdout().is_terminal()
        };

        if is_tty {
            // TODO: Check for pager in environment
            RenderMode::Progressive
        } else {
            RenderMode::Buffered
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_mode_detect_explicit_flags() {
        // --progressive (Some(true)) should force progressive mode
        assert_eq!(
            RenderMode::detect(Some(true), false),
            RenderMode::Progressive
        );
        assert_eq!(
            RenderMode::detect(Some(true), true),
            RenderMode::Progressive
        );

        // --no-progressive (Some(false)) should force buffered mode
        assert_eq!(RenderMode::detect(Some(false), false), RenderMode::Buffered);
        assert_eq!(RenderMode::detect(Some(false), true), RenderMode::Buffered);

        // None should auto-detect (tested via TTY checks in runtime)
    }
}
