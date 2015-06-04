extern crate nebchat;

use std::io;
use std::io::BufRead;
use std::env;
use std::thread;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
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
            let downstream = TcpStream::connect(address).unwrap();
            let upstream = downstream.try_clone().unwrap();

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
                loop_write(input_rx, upstream, Arc::new(AtomicBool::new(true)));
            });

            let input_thread = thread::spawn(move || {
                let stdin  = io::stdin();
                for line in stdin.lock().lines() {
                    input_tx.send(line.unwrap()).unwrap();
                }
            });

            input_thread.join().unwrap();
            output_thread.join().unwrap();
            down_thread.join().unwrap();
            up_thread.join().unwrap();
        },
        Err(msg) => {
            panic!("{:?}. Bad hostname?", msg);
        }
    };


}
