mod router;
mod packets;

use std::fs::read_to_string;

fn main() {
    let contents = read_to_string("test.tgsk").expect("Failed to read test.tgsk");
    let lines = contents.lines();

    for line in lines {
        router::route(line.trim());
    }
}
