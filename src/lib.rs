use std::io::{Write, BufRead, BufReader};
use std::string::String;
use std::thread;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::atomic::{AtomicBool, Ordering};
use std::net::{TcpListener, TcpStream};


const DELIMITER: u8 = b"\0"[0];

fn write(msg: String, stream: &mut TcpStream) {
    for byte in msg.into_bytes() {
        stream.write(&[byte]).unwrap();
    }
    stream.write(&[DELIMITER]).unwrap();
}

pub fn loop_write(input: Receiver<String>, mut stream: TcpStream, until: Arc<AtomicBool>) {
    for msg in input.iter() {
        if until.load(Ordering::SeqCst) {
            write(msg, &mut stream);
        } else {
            break;
        }
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
            Ok(_) => {
                message.pop();  // Strip the delimiter.
                let message_string = String::from_utf8(message).unwrap();
                output.send(message_string).unwrap();
            },
            _ => {
                println!("Something unexpected...");
            }
        };
    }
}

fn handle_client(downstream: TcpStream, up: Sender<String>, down: Receiver<String>) {
    let upstream = downstream.try_clone().unwrap();
    let alive = Arc::new(AtomicBool::new(true));
    let alive_copy = alive.clone();
    thread::spawn(move || {
        loop_read(up, upstream);
        alive_copy.store(false, Ordering::SeqCst);
    });

    let alive_copy_2 = alive.clone();
    let w = thread::spawn(move || {
        loop_write(down, downstream, alive_copy_2);
    });

    w.join().unwrap();
}

pub fn listen() {
    let (up_tx, up_rx) = channel::<String>();
    let listener = TcpListener::bind("0.0.0.0:9001").unwrap();

    let down_txs: HashMap<usize, Sender<String>> = HashMap::new();
    let down_txs_mutex = Arc::new(Mutex::new(down_txs));

    let down_txs_mutex_copy = down_txs_mutex.clone();
    thread::spawn(move || {
        for msg in up_rx.iter() {
            println!("{}", msg);
            let ref view = *down_txs_mutex_copy.lock().unwrap();
            for (_, down_tx) in view.iter() {
                down_tx.send(msg.clone()).unwrap();
            }
        }
    });

    for (id, stream) in listener.incoming().enumerate() {
        match stream {
            Ok(stream) => {
                println!("Client connected, assigned id {}.", id);
                let (down_tx, down_rx) = channel::<String>();
                (*down_txs_mutex.lock().unwrap()).insert(id, down_tx);
                let up_tx_copy = up_tx.clone();
                let h = thread::spawn(move || {
                    handle_client(stream, up_tx_copy, down_rx);
                });
                let down_txs_mutex_copy = down_txs_mutex.clone();
                thread::spawn(move || {
                    h.join().unwrap();
                    (*down_txs_mutex_copy.lock().unwrap()).remove(&id);
                    println!("Removed client {}.", id);
                });
            },
            Err(msg) => {
                println!("{}", msg);
            }
        }
    }
}
