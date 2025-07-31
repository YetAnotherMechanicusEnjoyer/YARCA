use std::{
    io::{self, Read, Write, stdout},
    net::TcpStream,
    thread,
};

fn main() -> io::Result<()> {
    print!("Enter server's ip address: ");
    stdout().flush()?;
    let mut ip = String::new();
    io::stdin().read_line(&mut ip)?;
    let ip = ip.trim().to_string();

    print!("Enter server's port: ");
    stdout().flush()?;
    let mut port = String::new();
    io::stdin().read_line(&mut port)?;
    let port = port.trim().to_string();

    print!("Enter your username: ");
    stdout().flush()?;
    let mut username = String::new();
    io::stdin().read_line(&mut username)?;
    let username = username.trim().to_string();

    let addr = format!("{ip}:{port}");

    let mut stream = TcpStream::connect(addr)?;
    println!("Connected to chat server !");

    stream.write_all(username.as_bytes())?;
    stream.write_all(b"\n")?;

    let mut stream_clone = stream.try_clone()?;

    thread::spawn(move || {
        let mut buffer = [0; 1024];
        loop {
            match stream_clone.read(&mut buffer) {
                Ok(bytes_read) if bytes_read > 0 => {
                    let message = String::from_utf8_lossy(&buffer[..bytes_read]);
                    print!("{message}");
                }
                Ok(_) => {
                    println!("Server disconnected.");
                    break;
                }
                Err(e) => {
                    eprintln!("Error reading from server: {e}");
                    break;
                }
            }
        }
    });

    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if input.trim() == "/quit" {
            println!("Disconnecting...");
            break;
        }
        stream.write_all(input.as_bytes())?;
    }

    Ok(())
}
