use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseBigFixedError {
    Empty,
    InvalidFormat,
    InvalidCharacter(char),
    FractionalDigitsExceedScale { found: usize, scale: u32 },
}

impl fmt::Display for ParseBigFixedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "input is empty"),
            Self::InvalidFormat => write!(f, "invalid decimal format"),
            Self::InvalidCharacter(ch) => write!(f, "invalid decimal character '{ch}'"),
            Self::FractionalDigitsExceedScale { found, scale } => {
                write!(f, "fractional digits {found} exceed fixed scale {scale}")
            }
        }
    }
}

impl std::error::Error for ParseBigFixedError {}
