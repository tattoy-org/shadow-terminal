//! # Shadow Terminal
//! A fully-functional, fully-rendered terminal purely in memory.
//!
//! Useful for terminal multiplexers (a la `tmux`, `zellij`) and end to end testing TUI
//! applications.
//!
//! There are 2 convenience modules for using this library: [`ActiveTerminal`] and
//! [`SteppableTerminal`]. The former is run in a thread and can only be interacted with through
//! channels, it's aimed more towards real world applications. Whilst the latter must be stepped
//! through and is aimed more at end to end testing.
//!
//! The underlying [`ShadowTerminal`] is also designed to be used directly, but requires a bit
//! more setup. See `ActiveTerminal` and `SteppableTerminal` to see how.

#![expect(clippy::pub_use, reason = "How else are you supposed re-export??")]

pub use termwiz;
pub use wezterm_term;

pub mod active_terminal;
pub mod errors;
/// How to send and manage the output of the shadow terminal.
pub mod output {
    pub mod foreign;
    pub mod native;
}
pub mod pty;
pub mod shadow_terminal;
pub mod steppable_terminal;

/// Tests
pub mod tests {
    pub mod helpers;
}

/// All the control signals
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Protocol {
    /// End all loops to allow graceful shutdown
    End,
    /// Resize the PTY and shadow terminal
    Resize {
        /// Width of the shadow terminal
        width: u16,
        /// Height of the shadow terminal
        height: u16,
    },
    /// Scrolling of the terminal scrollback
    Scroll(Scroll),
}

/// The various states of scrolling
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Scroll {
    /// Scroll the Wezterm terminal frontend up
    Up,
    /// Scroll the Wezterm terminal frontend down
    Down,
    /// Exit the scroll, returning the terminal to how it was before scrolling started.
    Cancel,
}
