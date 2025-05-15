#![allow(dead_code)]

use std::fmt;
use termion::color::{Bg, Fg, Reset, Rgb};

// ANSI color constants
pub const RED: Rgb = Rgb(255, 0, 0);
pub const GREEN: Rgb = Rgb(0, 255, 0);
pub const BLUE: Rgb = Rgb(0, 0, 255);
pub const YELLOW: Rgb = Rgb(255, 255, 0);
pub const MAGENTA: Rgb = Rgb(255, 0, 255);
pub const CYAN: Rgb = Rgb(0, 255, 255);
pub const WHITE: Rgb = Rgb(255, 255, 255);
pub const BLACK: Rgb = Rgb(0, 0, 0);
pub const GRAY: Rgb = Rgb(128, 128, 128);
pub const LIGHT_GRAY: Rgb = Rgb(192, 192, 192);
pub const DARK_GRAY: Rgb = Rgb(64, 64, 64);

// Colored text wrapper
pub struct Colored<D> {
    text: D,
    fg: Option<Rgb>,
    bg: Option<Rgb>,
}

impl<D> Colored<D> {
    pub fn new(text: D) -> Self {
        Self {
            text,
            fg: None,
            bg: None,
        }
    }

    pub fn fg(mut self, color: Rgb) -> Self {
        self.fg = Some(color);
        self
    }

    pub fn bg(mut self, color: Rgb) -> Self {
        self.bg = Some(color);
        self
    }
}

impl<D: fmt::Display> fmt::Display for Colored<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(fg) = self.fg {
            write!(f, "{}", Fg(fg))?;
        }
        if let Some(bg) = self.bg {
            write!(f, "{}", Bg(bg))?;
        }
        write!(f, "{}", self.text)?;
        write!(f, "{}", Fg(Reset))?;
        write!(f, "{}", Bg(Reset))?;
        Ok(())
    }
}

// Utility functions for common color operations
pub fn red<D: fmt::Display>(text: D) -> Colored<D> {
    Colored::new(text).fg(RED)
}

pub fn green<D: fmt::Display>(text: D) -> Colored<D> {
    Colored::new(text).fg(GREEN)
}

pub fn blue<D: fmt::Display>(text: D) -> Colored<D> {
    Colored::new(text).fg(BLUE)
}

pub fn yellow<D: fmt::Display>(text: D) -> Colored<D> {
    Colored::new(text).fg(YELLOW)
}

pub fn magenta<D: fmt::Display>(text: D) -> Colored<D> {
    Colored::new(text).fg(MAGENTA)
}

pub fn cyan<D: fmt::Display>(text: D) -> Colored<D> {
    Colored::new(text).fg(CYAN)
}

pub fn white<D: fmt::Display>(text: D) -> Colored<D> {
    Colored::new(text).fg(WHITE)
}

pub fn black<D: fmt::Display>(text: D) -> Colored<D> {
    Colored::new(text).fg(BLACK)
}

pub fn gray<D: fmt::Display>(text: D) -> Colored<D> {
    Colored::new(text).fg(GRAY)
}

// Colored formatting with custom colors
pub fn rgb<D: fmt::Display>(text: D, r: u8, g: u8, b: u8) -> Colored<D> {
    Colored::new(text).fg(Rgb(r, g, b))
}

// Helper for command output formatting
pub fn format_command(command: &str) -> String {
    format!("{}", cyan(command))
}

// Helper for error messages
pub fn format_error(message: &str) -> String {
    format!("{}", red(message))
}

// Helper for success messages
pub fn format_success(message: &str) -> String {
    format!("{}", green(message))
}

// Helper for warning messages
pub fn format_warning(message: &str) -> String {
    format!("{}", yellow(message))
}

// Helper for info messages
pub fn format_info(message: &str) -> String {
    format!("{}", blue(message))
}
