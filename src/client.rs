extern crate nebchat;

use std::io;
use std::io::BufRead;
use std::env;
use std::thread;
use std::sync::mpsc::channel;
use std::net::{SocketAddr, AddrParseError, TcpStream};

use nebchat::*;


fn main() {
    println!("Welcome to nebchat!");

    let host_string = env::args().nth(1).unwrap();
    let host: Result<SocketAddr, AddrParseError> = host_string.parse();
    match host {
        Ok(address) => {
            println!("Connecting to {}.", host_string);
            let downstream = TcpStream::connect(address)
                .ok().expect("Connect to host.");
            let upstream = downstream.try_clone()
                .ok().expect("Clone stream.");

            let (output_tx, output_rx) = channel::<String>();

            let down_thread = thread::spawn(move || {
                loop_read(output_tx, downstream);
            });

            let output_thread = thread::spawn(move || {
                for msg in output_rx.iter() {
                    println!("Incoming: {}", msg);
                }
            });

            let (input_tx, input_rx) = channel::<String>();

            let up_thread = thread::spawn(move || {
                loop_write(input_rx, upstream);
            });

            let input_thread = thread::spawn(move || {
                let stdin  = io::stdin();
                for line in stdin.lock().lines() {
                    input_tx.send(line.unwrap()).ok();
                }
            });

            input_thread.join().ok();
            output_thread.join().ok();
            down_thread.join().ok();
            up_thread.join().ok();
        },
        Err(msg) => {
            panic!("{:?}. Bad hostname?", msg);
        }
    };


}
