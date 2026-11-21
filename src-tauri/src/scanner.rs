use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use crate::domain::codes;
use crate::engine::EngineError;

fn error(code: &'static str, message: &str) -> EngineError {
    EngineError { code, message: message.to_string(), retryable: false }
}

/// Quotes group a path or argument; no shell expansion or environment parsing.
fn arguments(command: &str) -> Result<Vec<String>, EngineError> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut present = false;
    for ch in command.chars() {
        if ch.is_control() {
            return Err(error(codes::INTERNAL, "The scanner command contains a control character."));
        }
        if let Some(delimiter) = quote {
            if ch == delimiter { quote = None } else { current.push(ch) }
        } else if matches!(ch, '\'' | '"') {
            quote = Some(ch);
            present = true;
        } else if ch.is_whitespace() {
            if present {
                args.push(std::mem::take(&mut current));
                present = false;
            }
        } else {
            present = true;
            current.push(ch);
        }
    }
    if quote.is_some() {
        return Err(error(codes::INTERNAL, "The scanner command has an unmatched quote."));
    }
    if present { args.push(current) }
    if args.first().is_none_or(|executable| executable.trim().is_empty()) {
        return Err(error(codes::INTERNAL, "Antivirus scanner command is empty."));
    }
    Ok(args)
}

pub(super) async fn run(command: &str, file: &Path, deadline: Duration) -> Result<(), EngineError> {
    let args = arguments(command)?;
    let mut process = tokio::process::Command::new(&args[0]);
    process.args(&args[1..]).arg(file)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    process.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    let mut child = process.spawn().map_err(|_| error(codes::INTERNAL, "Could not launch the configured antivirus scanner."))?;
    match tokio::time::timeout(deadline, child.wait()).await {
        Ok(Ok(status)) if status.success() => Ok(()),
        Ok(Ok(_)) => Err(error(codes::SCANNER_FAILED, "The antivirus scanner returned a failure status. Review the scanner's own report.")),
        Ok(Err(_)) => Err(error(codes::SCANNER_FAILED, "The antivirus scanner could not be monitored.")),
        Err(_) => {
            let _ = child.kill().await;
            Err(error(codes::SCANNER_FAILED, "Antivirus scanner timed out."))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoted_paths_and_arguments_are_passed_literally() {
        assert_eq!(arguments(r#""C:\Program Files\Scanner\scan.exe" --scan "a b""#).unwrap(),
            vec![r"C:\Program Files\Scanner\scan.exe", "--scan", "a b"]);
        assert_eq!(arguments("scanner;command $(token)").unwrap(), vec!["scanner;command", "$(token)"]);
        assert!(arguments("\"unclosed").is_err());
        assert!(arguments("\"\"").is_err());
        assert!(arguments("scanner\ncommand").is_err());
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn a_hung_scanner_is_stopped_at_the_timeout() {
        let runtime = std::env::var("SystemRoot").unwrap();
        let command = format!(r#""{runtime}\System32\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -Command "Start-Sleep -Seconds 30""#);
        let started = std::time::Instant::now();
        let result = run(&command, Path::new("unused-test-path"), Duration::from_millis(100)).await.unwrap_err();
        assert_eq!(result.code, codes::SCANNER_FAILED);
        assert!(result.message.contains("timed out"));
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
