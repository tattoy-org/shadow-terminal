//! Define the structure of output for FFI and STDOUT users.

/// The parent type for all output
#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[non_exhaustive]
pub struct Output {
    /// The shadow terminal's width.
    pub width: usize,
    /// The shadow terminal's height.
    pub height: usize,
    /// A 1D array of all the cells in the shadow terminal's screen.
    pub cells: Vec<Cell>,
    /// The shadow terminal's cursor state.
    pub cursor: Cursor,
    /// The title of the terminal.
    pub title: String,
    /// Whether the terminal is in the primary (scrolling) mode or the alternate mode.
    pub mode: super::native::ScreenMode,
}

/// An individual cell in the shadow terminal's screen.
#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[non_exhaustive]
pub struct Cell {
    /// The text contents of the cell.
    pub text: String,
    /// The foreground colour of the cell.
    pub foreground: Color,
    /// The background colour of the cell.
    pub background: Color,
}

/// The colour of a cell's foreground or background.
#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[non_exhaustive]
pub enum Color {
    /// The colour is the default foreground or background colour.
    Default,
    /// The colour is from the terminal's palette.
    PaletteIndex(u8),
    /// A true RGB colour.
    TrueColor((f32, f32, f32)),
}

/// An individual cell in the shadow terminal's screen.
#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Cursor {
    /// Position of the cursor. 0-indexed. `0,0` is in the top-left.
    pub position: (usize, usize),
    /// The shape of the cursor, and if it's blinking or not.
    pub shape: Option<termwiz::surface::CursorShape>,
    /// Whether the cursor is visible or hidden.
    pub visibility: termwiz::surface::CursorVisibility,
}

impl From<termwiz::color::ColorAttribute> for Color {
    #[inline]
    fn from(attribute: termwiz::color::ColorAttribute) -> Self {
        match attribute {
            termwiz::color::ColorAttribute::TrueColorWithPaletteFallback(srgba_tuple, _)
            | termwiz::color::ColorAttribute::TrueColorWithDefaultFallback(srgba_tuple) => {
                Self::TrueColor((srgba_tuple.0, srgba_tuple.1, srgba_tuple.2))
            }
            termwiz::color::ColorAttribute::PaletteIndex(index) => Self::PaletteIndex(index),
            termwiz::color::ColorAttribute::Default => Self::Default,
        }
    }
}

impl Output {
    /// Convert native output to a structure more suitable for non-Rust users.
    ///
    /// # Errors
    /// If trying to convert the scrollback.
    #[inline]
    pub fn convert_to_foreign(
        native: super::native::CompleteSurface,
    ) -> Result<Self, crate::errors::ShadowTerminalError> {
        let super::native::CompleteSurface::Screen(mut screen) = native else {
            snafu::whatever!("Converting the scrollback hasn't been implemented yet");
        };

        let mut cells = Vec::<Cell>::new();
        for line in screen.surface.screen_cells() {
            for cell in line {
                cells.push(Cell {
                    text: cell.str().to_owned(),
                    background: cell.attrs().foreground().into(),
                    foreground: cell.attrs().background().into(),
                });
            }
        }

        screen.surface.title();

        Ok(Self {
            width: screen.surface.dimensions().0,
            height: screen.surface.dimensions().1,
            cells,
            cursor: Cursor {
                shape: screen.surface.cursor_shape(),
                visibility: screen.surface.cursor_visibility(),
                position: screen.surface.cursor_position(),
            },
            title: screen.surface.title().to_owned(),
            mode: screen.mode,
        })
    }
}
