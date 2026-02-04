//! Theme Colors for Ratatui
//! Usage:
//!   use ratatui::style::Color;
//!   let theme = Theme::dark();
//!   let primary_color = theme.primary;

use ratatui::style::Color;

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub primary: Color,
    pub secondary: Color,
    pub primary_background: Color,
    pub secondary_background: Color,
    pub accent: Color,
    pub warning: Color,
    pub error: Color,
    pub success: Color,
    pub foreground: Color,
    pub background: Color,
    pub surface: Color,
    pub panel: Color,
    pub boost: Color,
}

impl Theme {
    pub const fn dark() -> Self {
        Self {
            primary: Color::from_u32(0x00ffff),
            secondary: Color::from_u32(0x008888),
            primary_background: Color::from_u32(0x225555),
            secondary_background: Color::from_u32(0x113333),
            accent: Color::from_u32(0xffaa22),
            warning: Color::from_u32(0x226666),
            error: Color::from_u32(0xff0000),
            success: Color::from_u32(0x00ff00),
            foreground: Color::from_u32(0xeeeeee),
            background: Color::from_u32(0x111111),
            surface: Color::from_u32(0x222222),
            panel: Color::from_u32(0x333333),
            boost: Color::from_u32(0x444444),
        }
    }

    /// Lighten a color by blending with white
    /// factor should be between 0.0 (no change) and 1.0 (white)
    pub fn lighten(color: Color, factor: f32) -> Color {
        let factor = factor.clamp(0.0, 1.0);
        match color {
            Color::Rgb(r, g, b) => {
                let r = r as f32 + (255.0 - r as f32) * factor;
                let g = g as f32 + (255.0 - g as f32) * factor;
                let b = b as f32 + (255.0 - b as f32) * factor;
                Color::Rgb(r as u8, g as u8, b as u8)
            }
            _ => color,
        }
    }

    /// Darken a color by blending with black
    /// factor should be between 0.0 (no change) and 1.0 (black)
    pub fn darken(color: Color, factor: f32) -> Color {
        let factor = factor.clamp(0.0, 1.0);
        match color {
            Color::Rgb(r, g, b) => {
                let r = r as f32 * (1.0 - factor);
                let g = g as f32 * (1.0 - factor);
                let b = b as f32 * (1.0 - factor);
                Color::Rgb(r as u8, g as u8, b as u8)
            }
            _ => color,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lighten() {
        let black = Color::Rgb(0, 0, 0);
        let lightened = Theme::lighten(black, 0.5);
        assert_eq!(lightened, Color::Rgb(127, 127, 127));
    }

    #[test]
    fn test_darken() {
        let white = Color::Rgb(255, 255, 255);
        let darkened = Theme::darken(white, 0.5);
        assert_eq!(darkened, Color::Rgb(127, 127, 127));
    }
}
