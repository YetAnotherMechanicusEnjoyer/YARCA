use std::{
    io::{self, Read, Write, stdout},
    net::TcpStream,
    thread,
    time::Duration,
};

const RECONNECT_DELAY: u64 = 5;

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

    let mut current_stream: Option<TcpStream> = None;

    loop {
        while current_stream.is_none() {
            println!("Attempting to connect to {}...", &addr);
            match TcpStream::connect(&addr) {
                Ok(mut stream) => {
                    println!("Connected to {} !", &addr);

                    if let Err(e) = stream.write_all(username.as_bytes()) {
                        eprintln!("Error sending username: {e}");
                        eprintln!(
                            "Connection failed during initial handshake. Retrying in {RECONNECT_DELAY} seconds..."
                        );
                        thread::sleep(Duration::from_secs(RECONNECT_DELAY));
                        continue;
                    }
                    if let Err(e) = stream.write_all(b"\n") {
                        eprintln!("Error sending newline after username: {e}");
                        eprintln!(
                            "Connection failed during initial handshake. Retrying in {RECONNECT_DELAY} seconds..."
                        );
                        thread::sleep(Duration::from_secs(RECONNECT_DELAY));
                        continue;
                    }

                    current_stream = Some(stream);
                }
                Err(e) => {
                    eprintln!("Failed to connect to {}: {e}", &addr);
                    eprintln!("Retrying in {RECONNECT_DELAY} seconds...");
                    thread::sleep(Duration::from_secs(RECONNECT_DELAY));
                }
            }
        }

        let mut stream = current_stream.take().unwrap();

        let mut write_stream_clone = stream.try_clone()?;

        let read_thread_username = username.clone();
        let read_handle = thread::spawn(move || {
            let mut buffer = [0; 1024];
            loop {
                match write_stream_clone.read(&mut buffer) {
                    Ok(bytes_read) if bytes_read > 0 => {
                        let message = String::from_utf8_lossy(&buffer[..bytes_read]);
                        print!("{message}");
                    }
                    Ok(_) => {
                        println!("\nServer disconnected. Attempting to reconnect...");
                        break;
                    }
                    Err(ref e)
                        if e.kind() == io::ErrorKind::ConnectionReset
                            || e.kind() == io::ErrorKind::BrokenPipe
                            || e.kind() == io::ErrorKind::UnexpectedEof =>
                    {
                        eprintln!(
                            "\nConnection error for {read_thread_username}: {e}. Attempting to reconnect..."
                        );
                        break;
                    }
                    Err(e) => {
                        eprintln!(
                            "\nUnexpected error reading form server for {read_thread_username}: {e}. Attempting to reconnect..."
                        );
                        break;
                    }
                }
            }
        });

        let mut send_result = Ok(());

        loop {
            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(_) => {
                    if input.trim() == "/quit" {
                        println!("Disconnecting...");
                        let _ = stream.shutdown(std::net::Shutdown::Both);
                        send_result = Ok(());
                        break;
                    }

                    if let Err(e) = stream.write_all(input.as_bytes()) {
                        eprintln!("Error sending message: {e}");
                        send_result = Err(e);
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("Error reading from stdin: {e}");
                    break;
                }
            }
        }

        let _ = read_handle.join();

        if send_result.is_err() {
            println!("Disconnected from server. Retrying connection...");
        } else {
            break;
        }
    }

    Ok(())
}
