use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    style::Print,
    terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode},
};
use hex::{decode, encode};
use rand::{Rng, rng};
use std::{
    collections::HashMap,
    fmt,
    io::{self, Read, Write, stdout},
    net::TcpStream,
    sync::mpsc,
    thread,
    time::Duration,
};

const RECONNECT_DELAY: u64 = 5;

#[derive(PartialEq, Clone)]
enum ClientEvent {
    UserInput(String),
    ServerDisconnected,
    Custom(Command),
}

#[derive(PartialEq, Clone, Copy)]
enum Command {
    Help,
    Addr,
    Quit,
}

enum EventError {
    NotFound,
}

impl fmt::Display for ClientEvent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let desc = match *self {
            ClientEvent::UserInput(_) => "User's input",
            ClientEvent::ServerDisconnected => "No connexion with server",
            ClientEvent::Custom(cmd) => &(format!("{cmd}")),
        };
        f.write_str(desc)
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let desc = match *self {
            Command::Help => "Shows available commands",
            Command::Addr => "Shows server's address",
            Command::Quit => "Quit chat",
        };
        f.write_str(desc)
    }
}

impl fmt::Display for EventError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let desc = match *self {
            EventError::NotFound => "Command not found",
        };
        f.write_str(desc)
    }
}

fn commands(cmds_map: &HashMap<&str, ClientEvent>, cmd: &str) -> Result<ClientEvent, EventError> {
    if let Some(event) = cmds_map.get(&cmd) {
        Ok(event.to_owned())
    } else {
        Err(EventError::NotFound)
    }
}

fn help() {
    let cmds = init_hashmap();
    execute!(io::stdout(), Print("Available Commands :\n\r")).unwrap();
    for (cmd, desc) in cmds.iter() {
        execute!(io::stdout(), Print(format!("  {cmd} : {desc}\n\r"))).unwrap();
    }
}

fn encrypt(plaintext: &str, key: &[u8; 32]) -> (String, String) {
    let mut rng = rng();
    let nonce_bytes: [u8; 12] = rng.random();
    let cipher = Aes256Gcm::new_from_slice(key).expect("Cipher failed.");
    let nonce = Nonce::from_slice(&nonce_bytes);

    let cipher_text = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .expect("Encryption failed.");
    (encode(nonce_bytes), encode(&cipher_text))
}

fn decrypt(nonce_hex: &str, ciphertext_hex: &str, key: &[u8; 32]) -> Option<String> {
    let cipher = Aes256Gcm::new_from_slice(key).expect("Cipher failed.");

    let nonce_bytes = decode(nonce_hex).ok()?;
    let ciphertext_bytes = decode(ciphertext_hex).ok()?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    match cipher.decrypt(nonce, ciphertext_bytes.as_slice()) {
        Ok(plaintext_bytes) => String::from_utf8(plaintext_bytes).ok(),
        Err(_) => None,
    }
}

fn init_hashmap() -> HashMap<&'static str, ClientEvent> {
    let mut hashmap: HashMap<&'static str, ClientEvent> = HashMap::new();
    hashmap.insert("quit", ClientEvent::Custom(Command::Quit));
    hashmap.insert("help", ClientEvent::Custom(Command::Help));
    hashmap.insert("addr", ClientEvent::Custom(Command::Addr));
    hashmap
}

fn input_manager(
    input_buffer: &mut String,
    key_event: KeyEvent,
    cmds_map: &HashMap<&str, ClientEvent>,
) -> Option<ClientEvent> {
    match key_event.code {
        KeyCode::Enter => {
            if !input_buffer.is_empty() {
                let input = input_buffer.trim().to_string();
                input_buffer.clear();
                execute!(
                    io::stdout(),
                    cursor::MoveToColumn(0),
                    Clear(ClearType::CurrentLine)
                )
                .unwrap();
                if input.starts_with('/') {
                    let command = input.trim_start_matches('/');
                    execute!(io::stdout(), Print(format!("\n>>> {input}\n\r"))).unwrap();
                    match commands(cmds_map, command) {
                        Ok(event) => return Some(event),
                        Err(e) => {
                            execute!(
                                io::stdout(),
                                Print(format!("Command Error (\"{command}\"): {e}\n\r"))
                            )
                            .unwrap();
                            return None;
                        }
                    };
                }
                Some(ClientEvent::UserInput(input))
            } else {
                None
            }
        }
        KeyCode::Backspace => {
            if !input_buffer.is_empty() {
                input_buffer.pop();
                execute!(
                    io::stdout(),
                    cursor::MoveLeft(1),
                    Clear(ClearType::UntilNewLine)
                )
                .unwrap();
            }
            None
        }
        KeyCode::Char(c) => {
            input_buffer.push(c);
            execute!(io::stdout(), Print(c)).unwrap();
            None
        }
        _ => None,
    }
}

fn start_input() -> io::Result<(String, String)> {
    print!("Enter server's address: ");
    stdout().flush()?;
    let mut addr = String::new();
    io::stdin().read_line(&mut addr)?;
    let addr = addr.trim().to_string();

    print!("Enter your username: ");
    stdout().flush()?;
    let mut username = String::new();
    io::stdin().read_line(&mut username)?;
    let username = username.trim().to_string();

    Ok((addr, username))
}

fn main() -> io::Result<()> {
    dotenvy::dotenv().ok();

    let secret_key_string = std::env::var("SECRET").expect("SECRET must be set in the .env file");

    let secret_key: [u8; 32] = match secret_key_string.as_bytes().try_into() {
        Ok(key_bytes) => key_bytes,
        Err(_) => panic!("SECRET must be exactly 32 bytes long."),
    };

    let (addr, username) = start_input()?;

    let cmds_map = init_hashmap();

    enable_raw_mode().unwrap();

    struct TerminalGuard;
    impl Drop for TerminalGuard {
        fn drop(&mut self) {
            execute!(io::stdout(), cursor::MoveTo(0, 0), Clear(ClearType::All)).unwrap();
            disable_raw_mode().unwrap();
        }
    }
    let _guard = TerminalGuard;

    execute!(io::stdout(), cursor::MoveTo(0, 0), Clear(ClearType::All)).unwrap();

    let (tx_main_event, rx_main_event) = mpsc::channel::<ClientEvent>();

    let tx_stdin = tx_main_event.clone();
    thread::spawn(move || {
        let mut input_buffer = String::new();

        loop {
            if let Ok(Event::Key(key_event)) = event::read() {
                if key_event.kind == KeyEventKind::Press {
                    if let Some(event) = input_manager(&mut input_buffer, key_event, &cmds_map) {
                        let _ = tx_stdin.send(event.clone());
                        if event == ClientEvent::Custom(Command::Quit) {
                            break;
                        }
                    }
                }
            }
        }
    });

    'connection_loop: loop {
        let mut stream = loop {
            execute!(
                io::stdout(),
                Print(format!("Attempting to connect to {}...\n\r", &addr))
            )?;
            match TcpStream::connect(&addr) {
                Ok(mut s) => {
                    execute!(io::stdout(), Print(format!("Connected to {}\n\r", &addr)))?;

                    let (nonce, encrypted_username) = encrypt(&username, &secret_key);
                    let encrypted_message = format!("{nonce}:{encrypted_username}\n\r");
                    if let Err(e) = s.write_all(encrypted_message.as_bytes()) {
                        execute!(
                            io::stdout(),
                            Print(format!("Error sending username: {e}\n\r"))
                        )?;
                        execute!(
                            io::stdout(),
                            Print(format!(
                                "Connection failed during initial handshake. Retrying in {RECONNECT_DELAY} seconds...\n\r"
                            ))
                        )?;
                        thread::sleep(Duration::from_secs(RECONNECT_DELAY));
                        continue;
                    }

                    break s;
                }
                Err(e) => {
                    execute!(
                        io::stdout(),
                        Print(format!("Failed to connect to {}: {e}\n\r", &addr))
                    )?;
                    execute!(
                        io::stdout(),
                        Print(format!("Retrying in {RECONNECT_DELAY} seconds...\n\r"))
                    )?;
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
                        let received_data = String::from_utf8_lossy(&buffer[..bytes_read]);
                        if let Some((nonce_hex, ciphertext_hex)) = received_data.split_once(':') {
                            if let Some(decrypted_message) =
                                decrypt(nonce_hex, ciphertext_hex, &secret_key)
                            {
                                execute!(io::stdout(), Print(format!("\n{decrypted_message}\n\r")))
                                    .unwrap();
                            } else {
                                execute!(io::stdout(), Print("Failed to decrypt message.\n\r"))
                                    .unwrap();
                            }
                        } else {
                            execute!(
                                io::stdout(),
                                Print(format!("Received malformed message: {received_data}\n\r"))
                            )
                            .unwrap();
                        }
                    }
                    Ok(_) => {
                        println!("\nServer disconnected.");
                        let _ = tx_read_event.send(ClientEvent::ServerDisconnected);
                        break;
                    }
                    Err(ref e)
                        if e.kind() == io::ErrorKind::ConnectionReset
                            || e.kind() == io::ErrorKind::BrokenPipe
                            || e.kind() == io::ErrorKind::UnexpectedEof =>
                    {
                        execute!(
                            io::stdout(),
                            Print(format!(
                                "\nConnection error for {read_thread_username}: {e}. Attempting to reconnect...\n\r"
                            ))
                        ).unwrap();
                        let _ = tx_read_event.send(ClientEvent::ServerDisconnected);
                        break;
                    }
                    Err(e) => {
                        execute!(
                            io::stdout(),
                            Print(format!(
                                "\nUnexpected error reading from server for {read_thread_username}: {e}\n\r"
                            ))
                        ).unwrap();
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
                        let (nonce, encrypted_message) = encrypt(&input, &secret_key);
                        let message_to_send = format!("{nonce}:{encrypted_message}\n\r");
                        if let Err(e) = stream.write_all(message_to_send.as_bytes()) {
                            execute!(
                                io::stdout(),
                                Print(format!("Error sending message: {e}\n\r"))
                            )?;
                            let _ = tx_main_event.send(ClientEvent::ServerDisconnected);
                            break;
                        }
                    }
                    ClientEvent::ServerDisconnected => {
                        break;
                    }
                    ClientEvent::Custom(cmd) => match cmd {
                        Command::Help => {
                            help();
                        }
                        Command::Addr => {
                            execute!(io::stdout(), Print(format!("Server : {addr}\n\r")))?;
                        }
                        Command::Quit => {
                            execute!(io::stdout(), Print("\nDisconnecting...\n\r"))?;
                            let _ = stream.shutdown(std::net::Shutdown::Both);
                            break 'connection_loop;
                        }
                    },
                },
                Err(mpsc::TryRecvError::Empty) => {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    execute!(
                        io::stdout(),
                        Print("Event channel disconnected. Exiting client.\n\r")
                    )?;
                    break 'connection_loop;
                }
            }
        }

        let _ = read_handle.join();
    }

    Ok(())
}
