use crate::commands_registry::{Command, CommandType, register_command};
use crate::input_handler::autocomplete_empty;
use regex::Regex;
use std::process::Command as BashCommand;
use syntect::util::{LinesWithEndings, as_24_bit_terminal_escaped};

pub fn register_bash_command() {
    register_command(Command {
        name: "bash".to_string(),
        pattern: Regex::new(r"^>\s*(.+)").unwrap(),
        description: "Bash command".to_string(),
        usage_example: ">ls -alt".to_string(),
        handler: |params| {
            if params.len() < 1 {
                println!("Usage: > Bash command (eg. ls, cp , rm)");
                return Ok(None);
            }
            let cmd = &params[0];
            Ok(Some(execute_shell_command(cmd, true)))
        },
        section: "terminal".to_string(),
        command_type: CommandType::Terminal,
        autocomplete_handler: Some(autocomplete_empty),
    });
}

fn execute_shell_command(command_str: &str, use_bash: bool) -> String {
    let shell_executable = if use_bash { "bash" } else { "sh" };

    let output = BashCommand::new(shell_executable)
        .arg("-c") // Tells the shell to execute the following string
        .arg(command_str) // The actual command string
        .output(); // Executes and collects output

    match output {
        Ok(output) => {
            //println!("Status: {}", output.status);
            // Print STDOUT
            if !output.stdout.is_empty() {
                match String::from_utf8(output.stdout) {
                    Ok(stdout_str) => {
                        return format!("{}", highlight_bash_output(&stdout_str));
                    }
                    Err(e) => {
                        return format!("STDOUT is not valid UTF-8: {}", e);
                    }
                }
            }
            if !output.stderr.is_empty() {
                match String::from_utf8(output.stderr) {
                    Ok(stderr_str) => {
                        return format!("{}", highlight_bash_output(&stderr_str));
                    }
                    Err(e) => {
                        return format!("STDERR is not valid UTF-8: {}", e);
                    }
                }
            } else {
                if output.status.success() {
                    return format!("--- STDERR (empty) ---");
                } else {
                    return format!("");
                }
            }
        }
        Err(e) => {
            // This error means the shell itself (sh/bash) failed to start
            return format!("Failed to spawn shell process for command '{}'", e);
        }
    }
}

fn highlight_bash_output(code: &str) -> String {
    const CSI: &str = "\x1B[";
    const RESET_ALL: &str = "\x1B[0m";

    // Foreground Color (Standard White)
    const FG_WHITE_STANDARD: &str = "15";
    // OR: Foreground Color (8-bit Bright White - often index 15 or 231)
    // const FG_WHITE_8BIT: &str = "38;5;15"; // Using 38;5;{N} for foreground

    // Background Color (8-bit Grey - 234 is a dark grey)
    // The full sequence part for 8-bit background is "48;5;{N}"
    const BG_GREY_8BIT_CODE: &str = "240"; // Just the color code
    const BG_GREY_SGR_PARAM: &str = "48;5"; // The SGR command for 8-bit background

    // Erase line
    const ERASE_LINE: &str = "2K";

    // Construct the SGR parameters string for colors
    // Example: "48;5;240;37" for grey BG and standard white FG
    let color_params = format!(
        "{};{};{}", // SGR for BG ; SGR for FG color code
        BG_GREY_SGR_PARAM,
        BG_GREY_8BIT_CODE,
        FG_WHITE_STANDARD // Use standard white
                          // If you wanted 8-bit white:
                          // FG_WHITE_8BIT
    );

    let mut highlighted_code = String::new();

    for line in LinesWithEndings::from(code) {
        // LinesWithEndings enables use of newlines mode
        highlighted_code.push_str(&format!(
            "{}{}{}{}{}{}{}",
            CSI,          // Start SGR sequence
            color_params, // e.g., "48;5;234;37"
            "m",          // End SGR sequence
            CSI,          // Start Erase Line sequence
            ERASE_LINE,   // Erase the line (fills with newly set background)
            line,         // Your actual text
            RESET_ALL
        ));
    }
    highlighted_code
}
