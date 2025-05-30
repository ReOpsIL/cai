use lazy_static::lazy_static;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{LinesWithEndings, as_24_bit_terminal_escaped};
use crate::utils::terminal;

lazy_static! {
    pub static ref SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
    pub static ref THEME_SET: ThemeSet = ThemeSet::load_defaults();
}

const CSI: &str = "\x1B[";
const RESET_ALL: &str = "\x1B[0m";
const FG_BLACK: &str = "30";
const BG_WHITE: &str = "47"; // Or "47" for standard white
const ERASE_LINE: &str = "2K"; // Erases the entire line, cursor does not move (typically stays at column 1)

pub fn highlight_code(code: &str) -> String {
    let ss = &*SYNTAX_SET;
    let ts = &*THEME_SET;

    let mut highlighted_code = String::new();
    // Default to "py" like in the original code, can be made a parameter later if needed.
    let syntax = ss
        .find_syntax_by_extension("py") 
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    let mut h = HighlightLines::new(syntax, &ts.themes["InspiredGitHub"]);

    for line in LinesWithEndings::from(code) {
        // Pass `ss` (SyntaxSet) to highlight_line as per syntect docs & original logic.
        // The original code had `let ps = SyntaxSet::load_defaults_newlines();` inside
        // highlight_code, which was redundant as SYNTAX_SET is already loaded.
        let ranges: Vec<(Style, &str)> = h.highlight_line(line, ss).unwrap();
        let highlighted_line = as_24_bit_terminal_escaped(&ranges[..], true);
        let highlighted_line_wh_bg = terminal::white_bg(highlighted_line);
        highlighted_code.push_str(&format!(
            "{}{};{}m{}{}{}{}",
            CSI,      // Start sequence
            BG_WHITE, // Set background to white
            FG_BLACK, // Set foreground to black (separated by ';')
            CSI,
            ERASE_LINE,             // Erase the line (fills with current background)
            highlighted_line_wh_bg, // Your actual text
            RESET_ALL
        ));
    }

    highlighted_code
}
