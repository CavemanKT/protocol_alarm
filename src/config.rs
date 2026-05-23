use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

fn protocol_path() -> PathBuf {
    // Basic: current working dir + "protocol.txt"
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("protocol.txt")
}

pub fn load_protocol() -> String {
    let path = protocol_path();
    fs::read_to_string(&path).unwrap_or_else(|_| {
        // default content if file missing or unreadable
        r#"Protocol.

Follow only these rules.

Rule 1: Pay attention only when price is near the middle Bollinger Band.
Rule 2: Never chase overbought or oversold prices.
Rule 3: Always use a stop loss.
Rule 4: Never average down.
Rule 5: Stop trading when the account is down to 300 at 0.01 lot size, or 3000 at 0.3 lot size.

For clarity:
Step 1: Meditate using box breathing as often as possible.
Step 2: Spend the remaining time practicing guitar."#.to_string()
    })
}

pub fn save_protocol(contents: &str) -> io::Result<()> {
    let path = protocol_path();
    let mut file = fs::File::create(&path)?;
    file.write_all(contents.as_bytes())?;
    Ok(())
}