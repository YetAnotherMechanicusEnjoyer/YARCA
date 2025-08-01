use std::{
    io::{self, Read, Write, stdout},
    net::TcpStream,
    sync::mpsc,
    thread,
    time::Duration,
};

const RECONNECT_DELAY: u64 = 5;

enum ClientEvent {
    UserInput(String),
    ServerDisconnected,
    Reconnect,
    Quit,
}

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

    let _ = TcpStream::connect(&addr)?;
    println!("Connected to chat server !");

    let (tx_main_event, rx_main_event) = mpsc::channel::<ClientEvent>();

    let tx_stdin = tx_main_event.clone();
    thread::spawn(move || {
        let mut input_buffer = String::new();
        loop {
            input_buffer.clear();
            match io::stdin().read_line(&mut input_buffer) {
                Ok(_) => {
                    let input_trim: &str = input_buffer.trim();
                    match input_trim {
                        "/quit" => {
                            let _ = tx_stdin.send(ClientEvent::Quit);
                            break;
                        }
                        "/reconnect" => {
                            println!("Reconnecting...");
                            let _ = tx_stdin.send(ClientEvent::Reconnect);
                        }
                        _ => {
                            let _ = tx_stdin.send(ClientEvent::UserInput(input_trim.to_string()));
                        }
                    };
                }
                Err(e) => {
                    eprintln!("Error reading from stdin thread: {e}");
                    let _ = tx_stdin.send(ClientEvent::Quit);
                    break;
                }
            }
        }
    });

    'connection_loop: loop {
        let mut stream = loop {
            println!("Attempting to connect to {}...", &addr);
            match TcpStream::connect(&addr) {
                Ok(mut s) => {
                    println!("Connected to {}", &addr);

                    if let Err(e) = s.write_all(username.as_bytes()) {
                        eprintln!("Error sending username: {e}");
                        eprintln!(
                            "Connection failed during initial handshake. Retrying in {RECONNECT_DELAY} seconds..."
                        );
                        thread::sleep(Duration::from_secs(RECONNECT_DELAY));
                        continue;
                    }

                    if let Err(e) = s.write_all(b"\n") {
                        eprintln!("Error sending newline after username: {e}");
                        eprintln!(
                            "Connection failed during initial handshade. Retrying in {RECONNECT_DELAY} seconds..."
                        );
                        thread::sleep(Duration::from_secs(RECONNECT_DELAY));
                        continue;
                    }
                    break s;
                }
                Err(e) => {
                    eprintln!("Failed to connect to {}: {e}", &addr);
                    eprintln!("Retrying in {RECONNECT_DELAY} seconds...");
                    thread::sleep(Duration::from_secs(RECONNECT_DELAY));
                }
            }
        };

        let mut read_stream_clone = stream.try_clone()?;
        let tx_read_event = tx_main_event.clone();
        let read_thread_username = username.clone();

        let read_handle = thread::spawn(move || {
            let mut buffer = [0; 1024];
            loop {
                match read_stream_clone.read(&mut buffer) {
                    Ok(bytes_read) if bytes_read > 0 => {
                        let message = String::from_utf8_lossy(&buffer[..bytes_read]);
                        print!("{message}");
                    }
                    Ok(_) => {
                        println!("\nServer disconnected. Attempting to reconnect...");
                        let _ = tx_read_event.send(ClientEvent::ServerDisconnected);
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
                        let _ = tx_read_event.send(ClientEvent::ServerDisconnected);
                        break;
                    }
                    Err(e) => {
                        eprintln!(
                            "\nUnexpected error reading from server for {read_thread_username}: {e}"
                        );
                        let _ = tx_read_event.send(ClientEvent::ServerDisconnected);
                        break;
                    }
                }
            }
        });

        loop {
            match rx_main_event.try_recv() {
                Ok(event) => match event {
                    ClientEvent::UserInput(input) => {
                        let message_to_send = format!("{input}\n");
                        if let Err(e) = stream.write_all(message_to_send.as_bytes()) {
                            eprintln!("Error sending message: {e}");
                            let _ = tx_main_event.send(ClientEvent::ServerDisconnected);
                            break;
                        }
                    }
                    ClientEvent::ServerDisconnected => {
                        break;
                    }
                    ClientEvent::Reconnect => {
                        println!("Manually triggering a reconnection attempt...");
                        let _ = stream.shutdown(std::net::Shutdown::Both);
                        break;
                    }
                    ClientEvent::Quit => {
                        println!("Disconnecting...");
                        let _ = stream.shutdown(std::net::Shutdown::Both);
                        break;
                    }
                },
                Err(mpsc::TryRecvError::Empty) => {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    eprintln!("Event channel disconnected. Exiting client.");
                    break 'connection_loop;
                }
            }
        }

        let _ = read_handle.join();
    }

    Ok(())
}
