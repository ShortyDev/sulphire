use std::{env, panic};
use std::borrow::Borrow;
use std::io::{BufRead, BufReader, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::Mutex;
use std::thread::sleep;
use std::time::Duration;

use fancy_regex::Regex;
use lazy_static::lazy_static;
use threadpool::ThreadPool;
use uuid::Uuid;

lazy_static! {
    static ref SUBSCRIBERS: Mutex<Vec<Subscriber>> = Mutex::new(Vec::new());
    static ref CLOSED: Mutex<Vec<Subscriber>> = Mutex::new(Vec::new());
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        println!("Missing bind address argument (e.g. 127.0.0.1:3216 or default) and auth key (e.g. \"YETTBDYZGYSDBGULZNUKXHSTLWPKDYBJ\")");
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
    let listener = TcpListener::bind(bind_addr.replace("default", "0.0.0.0:3216"))?;
    println!("Listening on {}", listener.local_addr()?);
    panic::set_hook(Box::new(|info| {
        println!("{}", info);
    }));
    let pool = ThreadPool::new(4);
    pool.execute(|| {
        loop {
            clean();
            sleep(Duration::from_secs(10));
        }
    });
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        pool.execute(|| {
            handle_client(stream, auth_key);
        });
    }
    Ok(())
}

fn clean() {
    let dropped = CLOSED.lock().unwrap().clone();
    CLOSED.lock().unwrap().clear();
    SUBSCRIBERS.lock().unwrap().retain(|subscriber| {
        return !dropped.contains(subscriber);
    })
}

fn handle_client(mut stream: TcpStream, auth_key: &str) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    println!("New connection from {}", stream.peer_addr().unwrap());
    await_auth(&mut stream, &mut reader, auth_key);
    await_sub_channels(&mut stream, &mut reader);
    println!("Proceeding with connection from {}", stream.peer_addr().unwrap());
    loop {
        let mut buffer = String::new();
        match reader.read_line(&mut buffer) {
            Err(_e) => {
                println!("Connection closed by {}", stream.peer_addr().unwrap());
                break;
            }
            _ => {
                if buffer.trim().is_empty() {
                    break;
                }
                let channel = buffer.split(" ").nth(0).unwrap();
                let message = buffer.split(" ").skip(1).collect::<Vec<&str>>().join(" ");
                for subscriber in SUBSCRIBERS.lock().unwrap().iter_mut().filter(|sub| sub.channel.eq(&channel)) {
                    let send = channel.borrow().to_string() + " " + message.borrow();
                    match subscriber.stream.write(send.as_bytes()) {
                        Ok(_) => (),
                        Err(_e) => {
                            subscriber.stream.shutdown(Shutdown::Both).unwrap();
                            subscriber.channel.clear();
                            CLOSED.lock().unwrap().push(subscriber.clone());
                            println!("Connection closed by {}", subscriber.stream.peer_addr().unwrap());
                        }
                    }
                }
            }
        }
    }
}

fn await_auth(stream: &mut TcpStream, reader: &mut BufReader<TcpStream>, server_key: &str) {
    let mut buffer = String::new();
    match reader.read_line(&mut buffer) {
        Ok(_x) => {
            if buffer.trim() == server_key {
                println!("Authenticated connection from {}", stream.peer_addr().unwrap());
                stream.write(b"Authentication successful\r\n").unwrap();
            } else {
                println!("Authentication failed from {}", stream.peer_addr().unwrap());
                stream.write(b"Authentication failed\r\n").unwrap();
                panic!("Authentication failed");
            };
        }
        Err(_) => {
            println!("Read timeout for connection {}", stream.peer_addr().unwrap());
        }
    };
}

fn await_sub_channels(stream: &mut TcpStream, reader: &mut BufReader<TcpStream>) {
    let mut buffer = String::new();
    match reader.read_line(&mut buffer) {
        Ok(_x) => {
            let channels = buffer.trim().split(",");
            let mut subscribers = Vec::new();
            for channel in channels {
                if channel.is_empty() {
                    continue;
                }
                subscribers.push(Subscriber {
                    id: Uuid::new_v4(),
                    channel: channel.to_string(),
                    stream: stream.try_clone().unwrap(),
                });
            }
            SUBSCRIBERS.lock().unwrap().append(&mut subscribers);
        }
        Err(_) => {
            println!("Read timeout for connection {}", stream.peer_addr().unwrap());
        }
    };
}

pub(crate) struct Subscriber {
    pub(crate) id: Uuid,
    pub(crate) stream: TcpStream,
    pub(crate) channel: String,
}

impl PartialEq for Subscriber {
    fn eq(&self, other: &Subscriber) -> bool {
        self.id == other.id
    }
}

impl Clone for Subscriber {
    fn clone(&self) -> Subscriber {
        Subscriber {
            id: self.id,
            stream: self.stream.try_clone().unwrap(),
            channel: self.channel.clone(),
        }
    }
}