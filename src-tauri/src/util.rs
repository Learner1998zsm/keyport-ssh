use std::process::{Command, Output, Stdio};

use crate::error::{AppResult, IoResultExt};

pub fn run_output(command: &mut Command, context: &str) -> AppResult<Output> {
    command
        .stdin(Stdio::null())
        .output()
        .io_context(context.to_string())
}

pub fn combined_output(output: &Output) -> String {
    let mut text = String::new();
    if !output.stdout.is_empty() {
        text.push_str(&String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    truncate(text.trim(), 6000)
}

pub fn truncate(value: &str, max_chars: usize) -> String {
    let mut output = String::new();
    for (count, ch) in value.chars().enumerate() {
        if count >= max_chars {
            output.push_str("\n… (output truncated)");
            break;
        }
        output.push(ch);
    }
    output
}

/// Quote one argument for a POSIX remote shell.
pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_quote_handles_single_quotes() {
        assert_eq!(shell_quote("hello world"), "'hello world'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
        assert_eq!(shell_quote("$HOME; rm -rf /"), "'$HOME; rm -rf /'");
    }
}
