use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use std::io::{BufRead, BufReader};

fn main() {
    let mut child = Command::new("target\\release\\tagspeak_rs.exe")
        .arg("examples\\burn\\count_burn.tgsk")
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to start tagspeak_rs.exe");

    let stdout = child.stdout.take().expect("no stdout?");
    let reader = BufReader::new(stdout);

    // Spawn a thread to read and print live output
    let handle = thread::spawn(move || {
        for line in reader.lines() {
            if let Ok(l) = line {
                println!("[tagspeak] {l}");
            }
        }
    });

    // Let it run for 5 seconds
    thread::sleep(Duration::from_secs(5));

    // Kill the child process
    let _ = child.kill();
    println!("⏹️ Burn test ended after 5 seconds.");

    // Wait for output thread to finish
    let _ = handle.join();
}
