use std::{
    collections::HashMap,
    fmt,
    io::{self, Read, Write, stdout},
    net::TcpStream,
    sync::mpsc,
    thread,
    time::Duration,
};

use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
use hex::{decode, encode};
use rand::{Rng, rng};

const RECONNECT_DELAY: u64 = 5;

#[derive(PartialEq, Clone)]
enum ClientEvent {
    UserInput(String),
    ServerDisconnected,
    Quit,
}

enum EventError {
    NotFound,
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
    hashmap.insert("quit", ClientEvent::Quit);
    hashmap
}

fn main() -> io::Result<()> {
    dotenvy::dotenv().ok();

    let secret_key_string = std::env::var("SECRET").expect("SECRET must be set in the .env file");

    let secret_key: [u8; 32] = match secret_key_string.as_bytes().try_into() {
        Ok(key_bytes) => key_bytes,
        Err(_) => panic!("SECRET must be exactly 32 bytes long."),
    };

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

    let cmds_map = init_hashmap();

    let (tx_main_event, rx_main_event) = mpsc::channel::<ClientEvent>();

    let tx_stdin = tx_main_event.clone();
    thread::spawn(move || {
        let mut input_buffer = String::new();
        loop {
            input_buffer.clear();
            match io::stdin().read_line(&mut input_buffer) {
                Ok(_) => {
                    let input_trim: &str = input_buffer.trim();
                    let input = input_trim.trim_matches('/');
                    match input_trim.chars().next() {
                        Some('/') => match commands(&cmds_map, input) {
                            Ok(event) => {
                                let _ = tx_stdin.send(event.clone());
                                if event == ClientEvent::Quit {
                                    break;
                                }
                            }
                            Err(e) => {
                                eprintln!("Command Error (\"{input}\") : {e}");
                            }
                        },
                        Some(_) => {
                            let _ = tx_stdin.send(ClientEvent::UserInput(input_trim.to_string()));
                        }
                        None => {
                            break;
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

                    let (nonce, encrypted_username) = encrypt(&username, &secret_key);
                    let encrypted_message = format!("{nonce}:{encrypted_username}\n");
                    if let Err(e) = s.write_all(encrypted_message.as_bytes()) {
                        eprintln!("Error sending username: {e}");
                        eprintln!(
                            "Connection failed during initial handshake. Retrying in {RECONNECT_DELAY} seconds..."
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
                        let received_data = String::from_utf8_lossy(&buffer[..bytes_read]);
                        if let Some((nonce_hex, ciphertext_hex)) = received_data.split_once(':') {
                            if let Some(decrypted_message) =
                                decrypt(nonce_hex, ciphertext_hex, &secret_key)
                            {
                                println!("{decrypted_message}");
                            } else {
                                eprintln!("Failed to decrypt message.");
                            }
                        } else {
                            eprintln!("Received malformed message: {received_data}");
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
                        let (nonce, encrypted_message) = encrypt(&input, &secret_key);
                        let message_to_send = format!("{nonce}:{encrypted_message}\n");
                        if let Err(e) = stream.write_all(message_to_send.as_bytes()) {
                            eprintln!("Error sending message: {e}");
                            let _ = tx_main_event.send(ClientEvent::ServerDisconnected);
                            break;
                        }
                    }
                    ClientEvent::ServerDisconnected => {
                        break;
                    }
                    ClientEvent::Quit => {
                        println!("Disconnecting...");
                        let _ = stream.shutdown(std::net::Shutdown::Both);
                        break 'connection_loop;
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
