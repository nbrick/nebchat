use std::io::{Write, BufRead, BufReader};
use std::string::String;
use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::net::{TcpListener, TcpStream};


const DELIMITER: u8 = b"\0"[0];

fn write(msg: String, stream: &mut TcpStream) {
    println!("Writing message");
    for byte in msg.into_bytes() {
        stream.write(&[byte]).ok();
    }
    stream.write(&[DELIMITER]).ok();
}

pub fn loop_write(input: Receiver<String>, mut stream: TcpStream) {
    for msg in input.iter() {
        println!("In loop_write: {:?}", msg);
        write(msg, &mut stream);
    }
}

pub fn loop_read(output: Sender<String>, stream: TcpStream) {
    let mut reader = BufReader::new(stream);
    loop {
        let mut message: Vec<u8> = Vec::new();
        let reader_result = reader.read_until(DELIMITER, &mut message);
        match reader_result {
            Ok(0) => {  // No bytes were sent.
                println!("Socket closed.");
                break;
            },
            Ok(n) => {
                let message_string = String::from_utf8(message).unwrap();
                println!("Read {} bytes: {:?}", n, message_string);
                output.send(message_string).ok();
            },
            _ => {
                println!("Something unexpected...");
            }
        };
    }
}

fn handle_client(downstream: TcpStream, up: Sender<String>, down: Receiver<String>) {
    println!("Client connected.");

    let upstream = downstream.try_clone()
        .ok().expect("Clone stream.");
    thread::spawn(move || {
        loop_read(up, upstream);
    });

    thread::spawn(move || {
        loop_write(down, downstream);
    });
}

pub fn listen() {
    let (up_tx, up_rx) = channel::<String>();
    let listener = TcpListener::bind("0.0.0.0:9001").unwrap();

    let down_txs: Vec<Sender<String>> = Vec::new();
    let down_txs_mutex = Arc::new(Mutex::new(down_txs));

    let down_txs_mutex_copy = down_txs_mutex.clone();
    thread::spawn(move || {
        for msg in up_rx.iter() {
            println!("{}", msg);
            let ref view = *down_txs_mutex_copy.lock().unwrap();
            for down_tx in view {
                println!("Emit to down thread {:?}", msg.clone());
                down_tx.send(msg.clone()).ok();
            }
        }
    });

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let (down_tx, down_rx) = channel::<String>();
                (*down_txs_mutex.lock().unwrap()).push(down_tx);
                let up_tx_copy = up_tx.clone();
                thread::spawn(move || {
                    handle_client(stream, up_tx_copy, down_rx);
                });
            },
            Err(msg) => {
                println!("{}", msg);
            }
        }
    }
}