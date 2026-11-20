//! A convenience module wrapping [`ShadowTerminal`] for running an active shadow terminal
//! running in a Tokio task.
//!
//! The underlying [`Wezterm`] terminal cannnot be interacted with directly. Instead input
//! and output must be sent and read over channels. This module is more likely useful for
//! real-world usecases, such as terminal multiplexing for example.

use tracing::Instrument as _;

/// An active terminal is running in a Tokio task, so we don't have direct access to the
/// underlying `wezterm_term::Terminal`. Instead we interact with it and the PTY through Tokio
/// channels.
#[non_exhaustive]
pub struct ActiveTerminal {
    /// The task handle to the actively running [`crate::shadow_tty::ShadowTerminal`].
    pub task_handle: tokio::task::JoinHandle<()>,
    /// A Tokio channel that receives [`termwiz::surface::Surface`] updates of the underlying
    /// terminal.
    pub surface_output_rx: tokio::sync::mpsc::Receiver<crate::output::native::Output>,
    /// A Tokio channel that forwards bytes to the underlying PTY's STDIN.
    pub pty_input_tx: Option<tokio::sync::mpsc::Sender<bytes::Bytes>>,
    /// A Tokio channel that provides PTY-like output bytes to the Shadow Terminal. Used for when
    /// the library user provides their own PTY-like process.
    pub ptylike_output_tx: Option<tokio::sync::mpsc::Sender<bytes::Bytes>>,
    /// A Tokio broadcast sender to send protocol messages that control the shadow terminal and
    /// PTY. For example; resizing and shutting down.
    pub control_tx: tokio::sync::broadcast::Sender<crate::Protocol>,
}

impl ActiveTerminal {
    /// Start a [`crate::shadow_tty::ShadowTerminal`] running in a Tokio task backed by a PTY.
    ///
    /// # Errors
    /// If starting the PTY errors.
    #[inline]
    pub fn start_with_pty(
        config: crate::shadow_terminal::Config,
    ) -> Result<Self, crate::errors::SteppableTerminalError> {
        tracing::debug!("Starting shadow terminal with PTY...");
        let (pty_input_tx, pty_input_rx) = tokio::sync::mpsc::channel(1);
        let (surface_output_tx, surface_output_rx) = tokio::sync::mpsc::channel(1);

        let mut shadow_terminal =
            crate::shadow_terminal::ShadowTerminal::new(config, None, surface_output_tx);
        let control_tx = shadow_terminal.channels.control_tx.clone();

        shadow_terminal.start_pty(pty_input_rx)?;

        let current_span = tracing::Span::current();
        let task_handle = tokio::spawn(async move {
            shadow_terminal.run().instrument(current_span).await;
        });
        tracing::debug!("Shadow terminal started.");

        Ok(Self {
            task_handle,
            surface_output_rx,
            pty_input_tx: Some(pty_input_tx),
            ptylike_output_tx: None,
            control_tx,
        })
    }

    /// Start a [`crate::shadow_tty::ShadowTerminal`] running in a Tokio task. It consumes PTY-like
    /// output from the caller.
    #[inline]
    #[must_use]
    pub fn start_without_pty(config: crate::shadow_terminal::Config) -> Self {
        tracing::debug!("Starting shadow terminal without PTY...");
        let (ptylike_output_tx, ptylike_output_rx) = tokio::sync::mpsc::channel(1);
        let (surface_output_tx, surface_output_rx) = tokio::sync::mpsc::channel(1);

        let mut shadow_terminal = crate::shadow_terminal::ShadowTerminal::new(
            config,
            Some(ptylike_output_rx),
            surface_output_tx,
        );
        let control_tx = shadow_terminal.channels.control_tx.clone();

        let current_span = tracing::Span::current();
        let task_handle = tokio::spawn(async move {
            shadow_terminal.run().instrument(current_span).await;
        });
        tracing::debug!("Shadow terminal started.");

        Self {
            task_handle,
            surface_output_rx,
            pty_input_tx: None,
            ptylike_output_tx: Some(ptylike_output_tx),
            control_tx,
        }
    }

    /// Advance the Shadow Terminal's state using PTY-like bytes. Likely only useful if you are
    /// providing your own PTY output.
    ///
    /// # Errors
    /// If sending the bytes fails.
    #[inline]
    pub async fn advance_bytes<T>(&self, bytes: T) -> Result<(), String>
    where
        T: Into<bytes::Bytes>,
    {
        let Some(sender) = &self.ptylike_output_tx else {
            return Err("Shadow Terminal already has a PTY to receive output from".to_owned());
        };

        sender
            .send(bytes.into())
            .await
            .map_err(|error| format!("Failed to send PTY bytes to Shadow Terminal: {error}"))
    }

    /// Send input directly into the underlying PTY process. This doesn't go through the shadow
    /// terminal's "frontend".
    ///
    /// # Errors
    /// If sending the bytes fails.
    #[inline]
    pub async fn send_input<T>(&self, bytes: T) -> Result<(), String>
    where
        T: Into<bytes::Bytes>,
    {
        let Some(sender) = &self.pty_input_tx else {
            return Err("Shadow Terminal does not have a PTY to receive input for".to_owned());
        };
        sender
            .send(bytes.into())
            .await
            .map_err(|error| format!("Failed to send input to PTY: {error}"))
    }

    /// End all loops and send OS kill signals to the underlying PTY.
    ///
    /// # Errors
    /// If sending message over channel fails.
    #[inline]
    pub fn kill(&self) -> Result<usize, tokio::sync::broadcast::error::SendError<crate::Protocol>> {
        tracing::debug!("`kill()` called on `ActiveTerminal`");
        self.control_tx.send(crate::Protocol::End)
    }

    /// Resize the shadow terminal "frontend". The PTY is agnostic about size.
    ///
    /// # Errors
    /// If sending message over channel fails.
    #[inline]
    pub fn resize(
        &self,
        width: u16,
        height: u16,
    ) -> Result<usize, tokio::sync::broadcast::error::SendError<crate::Protocol>> {
        self.control_tx
            .send(crate::Protocol::Resize { width, height })
    }

    /// Scroll the shadow Wezterm terminal up.
    ///
    /// # Errors
    /// If sending message over channel fails.
    #[inline]
    pub fn scroll_up(
        &self,
    ) -> Result<usize, tokio::sync::broadcast::error::SendError<crate::Protocol>> {
        self.control_tx
            .send(crate::Protocol::Scroll(crate::Scroll::Up))
    }

    /// Scroll the shadow Wezterm terminal down.
    ///
    /// # Errors
    /// If sending message over channel fails.
    #[inline]
    pub fn scroll_down(
        &self,
    ) -> Result<usize, tokio::sync::broadcast::error::SendError<crate::Protocol>> {
        self.control_tx
            .send(crate::Protocol::Scroll(crate::Scroll::Down))
    }

    /// Cancel scrolling, and return the scroll to normal.
    ///
    /// # Errors
    /// If sending message over channel fails.
    #[inline]
    pub fn scroll_cancel(
        &self,
    ) -> Result<usize, tokio::sync::broadcast::error::SendError<crate::Protocol>> {
        self.control_tx
            .send(crate::Protocol::Scroll(crate::Scroll::Cancel))
    }
}

impl Drop for ActiveTerminal {
    #[inline]
    fn drop(&mut self) {
        let result = self.kill();
        if let Err(error) = result {
            tracing::debug!("`ActiveTerminal.drop()`: {error:?}");
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[cfg(not(target_os = "windows"))]
    #[tokio::test(flavor = "multi_thread")]
    async fn basic_output() {
        let config = crate::shadow_terminal::Config {
            width: 10,
            height: 3,
            command: vec![],
            scrollback_size: 10,
            scrollback_step: 1,
        };
        let mut terminal = ActiveTerminal::start_without_pty(config);

        let move_to_next_line = "\x1b[E";
        let text = format!("{move_to_next_line}hello");
        let buffer = bytes::BytesMut::from(text.as_str());
        if let Some(pty) = &terminal.ptylike_output_tx {
            pty.send(buffer.into()).await.unwrap();
        }

        let output = terminal.surface_output_rx.recv().await.unwrap();
        let actual = match output {
            crate::output::native::Output::Diff(_) => panic!(),
            crate::output::native::Output::Complete(complete_surface) => match complete_surface {
                crate::output::native::CompleteSurface::Scrollback(_) => panic!(),
                crate::output::native::CompleteSurface::Screen(complete_screen) => {
                    complete_screen.surface.screen_chars_to_string()
                }
            },
        };

        assert_eq!(actual, "          \nhello     \n          \n");
    }
}
