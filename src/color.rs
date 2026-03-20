//! ANSI color codes for terminal output.
//! All output uses a hardcoded Catppuccin Macchiato palette (truecolor).

pub const RESET: &str = "\x1b[0m";
pub const PURPLE: &str = "\x1b[38;2;198;160;246m"; // mauve
pub const GREEN: &str = "\x1b[38;2;166;218;149m"; // green
pub const RED: &str = "\x1b[38;2;237;135;150m"; // red
pub const BLUE: &str = "\x1b[38;2;138;173;244m"; // blue
pub const YELLOW: &str = "\x1b[38;2;238;212;159m"; // yellow
pub const BRIGHT_MAGENTA: &str = "\x1b[38;2;245;189;230m"; // pink
pub const BRIGHT_BLACK: &str = "\x1b[38;2;110;115;141m"; // overlay0

// Catppuccin Macchiato palette aliases used by git-style output
pub const CAT_MAUVE: &str = "\x1b[38;2;198;160;246m";
pub const CAT_YELLOW: &str = "\x1b[38;2;238;212;159m";
pub const CAT_PEACH: &str = "\x1b[38;2;245;169;127m";
pub const CAT_MAROON: &str = "\x1b[38;2;238;153;160m";
pub const CAT_SUBTEXT0: &str = "\x1b[38;2;165;173;203m";
pub const CAT_RED: &str = "\x1b[38;2;237;135;150m";
pub const CAT_GREEN: &str = "\x1b[38;2;166;218;149m";
