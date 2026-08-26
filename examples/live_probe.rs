//! Deep diagnostic: separates SOCKET reachability from TLS/client behavior.
//! Debug aid only. Run while a real game is open.

use std::error::Error;
use std::net::TcpStream;
use std::time::Duration;

fn main() {
    // 1) Raw TCP: does ANY local process reach the game client's socket?
    match TcpStream::connect("127.0.0.1:2999") {
        Ok(stream) => println!("TCP CONNECT: OK (peer {:?})", stream.peer_addr()),
        Err(e) => {
            println!("TCP CONNECT FAILED: {e}");
            return;
        }
    }

    // 2) Full reqwest stack with our exact builder options, full error chain.
    let root = match reqwest::tls::Certificate::from_pem(include_bytes!("../assets/riotgames.pem")) {
        Ok(r) => r,
        Err(e) => {
            println!("PEM PARSE FAILED: {e}");
            return;
        }
    };
    let http = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(800))
        .tls_built_in_root_certs(false)
        .add_root_certificate(root)
        .build()
        .expect("builder");
    println!("REQWEST GET ...");
    match http.get("https://127.0.0.1:2999/liveclientdata/allgamedata").send() {
        Ok(resp) => println!("REQWEST OK: status {}", resp.status()),
        Err(e) => {
            println!("REQWEST ERR: {e}");
            let mut current = e.source();
            let mut level = 1;
            while let Some(src) = current {
                println!("  L{level}: {src}");
                current = src.source();
                level += 1;
            }
        }
    }
}
