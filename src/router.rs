use crate::packets;

pub fn route(packet_chain: &str) {
    let packets: Vec<&str> = packet_chain.split('>').collect();

    // Start with the first packet
    let mut result = match parse_and_run(packets[0].trim(), None) {
        Some(res) => res,
        None => return,
    };

    // Chain through remaining packets
    for pkt in packets.iter().skip(1) {
        result = match parse_and_run(pkt.trim(), Some(&result)) {
            Some(res) => res,
            None => return,
        };
    }
}

// Parses and runs individual packets with optional input
fn parse_and_run(packet: &str, input: Option<&str>) -> Option<String> {
    if packet.starts_with('[') && packet.ends_with(']') {
        let inner = &packet[1..packet.len() - 1];
        if let Some((op, arg)) = inner.split_once('@') {
            match op {
                "math" => Some(packets::math::run(arg)),        // output a string like "15"
                "print" => {
                    packets::print::run(input.unwrap_or(arg));  // input wins if available
                    Some(input.unwrap_or(arg).to_string())
                }
                _ => {
                    println!("(warn) unknown operation: [{}]", op);
                    None
                }
            }
        } else {
            // Handle packets without @ like [print]
            match inner {
                "print" => {
                    if let Some(value) = input {
                        packets::print::run(value);
                        Some(value.to_string())
                    } else {
                        println!("(warn) [print] has no input");
                        None
                    }
                }
                _ => {
                    println!("(error) malformed packet: {}", packet);
                    None
                }
            }
        }
    } else {
        println!("(error) invalid packet format: {}", packet);
        None
    }
}
