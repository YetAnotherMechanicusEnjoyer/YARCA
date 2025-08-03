use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread::spawn,
};

use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
use hex::{decode, encode};
use rand::{Rng, rng};

#[derive(Debug)]
enum ServerMessage {
    NewClient(String, Arc<Mutex<TcpStream>>),
    ClientDisconnected(String),
    ChatMessage(String, String),
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

fn main() -> Result<(), std::io::Error> {
    dotenvy::dotenv().ok();

    let secret_key_string = std::env::var("SECRET").expect("SECRET must be set in the .env file");
    let secret_key: [u8; 32] = match secret_key_string.as_bytes().try_into() {
        Ok(key_bytes) => key_bytes,
        Err(_) => panic!("SECRET must be exactly 32 bytes long."),
    };

    let addr = std::env::var("ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let listener = TcpListener::bind(&addr)?;
    println!("Server listening on {}", &addr);

    let (tx_server, rx_server) = std::sync::mpsc::channel::<ServerMessage>();
    let clients: Arc<Mutex<HashMap<String, Arc<Mutex<TcpStream>>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let clients_clone = clients.clone();
    let secret_key_arc = Arc::new(secret_key);
    let secret_key_arc_clone = secret_key_arc.clone();

    spawn(move || {
        for msg in rx_server {
            match msg {
                ServerMessage::NewClient(username, stream) => {
                    println!("Client {username} connected.");
                    clients_clone
                        .lock()
                        .unwrap()
                        .insert(username.clone(), stream);

                    let join_msg = format!("{username} has joined chat.");
                    for (name, client_stream_mutex) in clients_clone.lock().unwrap().iter() {
                        if name == &username {
                            continue;
                        }
                        let (nonce, encrypted_msg) = encrypt(&join_msg, &secret_key_arc_clone);
                        let message_to_send = format!("{nonce}:{encrypted_msg}");

                        let mut client_stream = client_stream_mutex.lock().unwrap();

                        let _ = client_stream.write_all(message_to_send.as_bytes());
                    }
                }
                ServerMessage::ClientDisconnected(username) => {
                    println!("Client {username} disconnected.");
                    clients_clone.lock().unwrap().remove(&username);

                    let disconnected_msg = format!("{username} has left chat.");

                    for (_, client_stream_mutex) in clients_clone.lock().unwrap().iter() {
                        let (nonce, encrypted_msg) =
                            encrypt(&disconnected_msg, &secret_key_arc_clone);
                        let message_to_send = format!("{nonce}:{encrypted_msg}");

                        let mut client_stream = client_stream_mutex.lock().unwrap();

                        let _ = client_stream.write_all(message_to_send.as_bytes());
                    }
                }
                ServerMessage::ChatMessage(sender, content) => {
                    let full_message = format!("[{sender}]: {content}");
                    println!("Broadcasting: {}", full_message.trim());

                    for (name, client_stream_mutex) in clients_clone.lock().unwrap().iter() {
                        if name == &sender {
                            continue;
                        }
                        let (nonce, encrypted_msg) = encrypt(&full_message, &secret_key_arc_clone);
                        let message_to_send = format!("{nonce}:{encrypted_msg}");

                        let mut client_stream = client_stream_mutex.lock().unwrap();

                        let _ = client_stream.write_all(message_to_send.as_bytes());
                    }
                }
            }
        }
    });

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let tx_clone = tx_server.clone();
                let secret_key_clone_for_handler = secret_key_arc.clone();

                let client_ip = stream.peer_addr().unwrap().to_string();
                println!("New connection {client_ip}");

                spawn(move || {
                    let mut stream_clone = stream.try_clone().expect("Failed to clone stream");
                    let arc_stream = Arc::new(Mutex::new(stream));

                    let mut buffer = [0; 1024];
                    let username;

                    match stream_clone.read(&mut buffer) {
                        Ok(bytes_read) if bytes_read > 0 => {
                            let msg = String::from_utf8_lossy(&buffer[..bytes_read]);
                            let (nonce_hex, ciphertext_hex) =
                                msg.trim().split_once(':').unwrap_or(("", ""));

                            match decrypt(nonce_hex, ciphertext_hex, &secret_key_clone_for_handler)
                            {
                                Some(name) => {
                                    username = name.trim().to_string();
                                    let _ = tx_clone.send(ServerMessage::NewClient(
                                        username.clone(),
                                        arc_stream.clone(),
                                    ));
                                }
                                None => {
                                    eprintln!("Failed to decrypt username. Disconnecting client.");
                                    return;
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error reading initial username from {client_ip} {e}");
                            return;
                        }
                        _ => {
                            eprintln!("Client {client_ip} disconnected before sending username.");
                            return;
                        }
                    }

                    loop {
                        match stream_clone.read(&mut buffer) {
                            Ok(bytes_read) if bytes_read > 0 => {
                                let message_content =
                                    String::from_utf8_lossy(&buffer[..bytes_read]);

                                let (nonce_hex, ciphertext_hex) =
                                    message_content.trim().split_once(':').unwrap_or(("", ""));

                                if let Some(decrypted_message) = decrypt(
                                    nonce_hex,
                                    ciphertext_hex,
                                    &secret_key_clone_for_handler,
                                ) {
                                    let _ = tx_clone.send(ServerMessage::ChatMessage(
                                        username.clone(),
                                        decrypted_message.trim().to_string(),
                                    ));
                                } else {
                                    eprintln!(
                                        "Failed to decrypt message from {username}. Dropping message."
                                    )
                                }
                            }
                            Ok(_) => {
                                println!("Client {username} disconnected.");
                                let _ = tx_clone
                                    .send(ServerMessage::ClientDisconnected(username.clone()));
                                break;
                            }
                            Err(e) => {
                                eprintln!("Error reading from client {username}: {e}");
                                let _ = tx_clone
                                    .send(ServerMessage::ClientDisconnected(username.clone()));
                                break;
                            }
                        }
                    }
                });
            }
            Err(e) => {
                eprintln!("Error accepting connection: {e}");
            }
        }
    }
    Ok(())
}
