use std::env;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::Command;

use fancy_regex::Regex;
use threadpool::ThreadPool;
use wait_timeout::ChildExt;

fn main() -> std::io::Result<()> { // test auth key YETTBDYZGYSDBGULZNUKXHSTLWPKDYBJ
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        println!("Missing bind address argument (e.g. 127.0.0.1:3216) and auth key (e.g. \"YETTBDYZGYSDBGULZNUKXHSTLWPKDYBJ\")");
        return Ok(());
    }
    let bind_addr = &args[1];
    let auth_key = Box::leak(Box::new(env::args().nth(2).unwrap()));
    if auth_key.len() >= 32 {
        let re = Regex::new(r"(.)\1{4,}").unwrap();
        if re.is_match(&*auth_key).unwrap() {
            println!("Auth key has too many matching consecutive characters");
            return Ok(());
        }
    } else {
        println!("Auth key is too short");
        return Ok(());
    }
    let listener = TcpListener::bind(bind_addr)?;
    println!("Listening on {}", bind_addr);
    let pool = ThreadPool::new(4);
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        pool.execute(|| {
            handle_client(stream, auth_key);
        });
    }
    Ok(())
}

fn handle_client(mut stream: TcpStream, auth_key: &str) {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(1))).unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    println!("New connection from {}", stream.peer_addr().unwrap());
    if await_auth(&mut stream, &mut reader, auth_key) {
        stream.set_read_timeout(None).unwrap();
    }
}

fn await_auth(stream: &mut TcpStream, reader: &mut BufReader<TcpStream>, server_key: &str) -> bool {
    let mut buffer = String::new();
    match reader.read_line(&mut buffer) {
        Ok(x) => {
            return if buffer.trim() == server_key {
                println!("Authenticated connection from {}", stream.peer_addr().unwrap());
                stream.write(b"Authentication successful\n").unwrap();
                true
            } else {
                println!("Authentication failed from {}", stream.peer_addr().unwrap());
                stream.write(b"Authentication failed\n").unwrap();
                false
            };
        }
        Err(_) => {
            println!("Read timeout for connection {}", stream.peer_addr().unwrap());
        }
    };
    true
}