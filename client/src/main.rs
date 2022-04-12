use std::env;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

fn main() -> std::io::Result<()> { // client for performance testing purposes only
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        println!("Missing connect address argument (e.g. 127.0.0.1:3216) and auth key (e.g. \"YETTBDYZGYSDBGULZNUKXHSTLWPKDYBJ\")");
        return Ok(());
    }
    let connect_addr = &args[1];
    let auth_key = Box::leak(Box::new(env::args().nth(2).unwrap()));
    let subscribe_to = &mut String::from("test");
    match TcpStream::connect(connect_addr) {
        Ok(mut stream) => {
            stream.write_all(&(auth_key.to_owned() + "\r\n").as_bytes())?;
            stream.write_all(&(subscribe_to.to_owned() + "\r\n").as_bytes())?;
            let mut reads = 0;
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            loop {
                let mut buffer = String::new();
                reader.read_line(&mut buffer)
                    .expect("Failed to read from stream");
                if buffer.is_empty() {
                    break;
                }
                reads += 1;
                println!("{}", reads);
            }
        }
        Err(e) => println!("{}", e),
    }
    return Ok(());
}
