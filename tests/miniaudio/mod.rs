mod enumeration;
mod playback_sine;

/// Shows a prompt and waits for input on stdin.
pub fn wait_for_enter() {
    use std::io::Write;

    print!("Press ENTER/RETURN to exit...");
    // Make sure the line above is displayed:
    std::io::stdout().flush().expect("failed to flush stdout");
    // Just read some random line off of stdin and discard it:
    std::io::stdin()
        .read_line(&mut String::new())
        .expect("failed to wait for line");
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
