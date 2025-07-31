use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread::spawn,
};

#[derive(Debug)]
enum ServerMessage {
    NewClient(String, Arc<Mutex<TcpStream>>),
    ClientDisconnected(String),
    ChatMessage(String, String),
}

fn main() -> Result<(), std::io::Error> {
    dotenvy::dotenv().ok();
    let ip = std::env::var("IP").unwrap_or_else(|_| "0.0.0.0".into());
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let addr = format!("{ip}:{port}");
    let listener = TcpListener::bind(&addr)?;
    println!("Server listening on {}", &addr);

    let (tx_server, rx_server) = std::sync::mpsc::channel::<ServerMessage>();
    let clients: Arc<Mutex<HashMap<String, Arc<Mutex<TcpStream>>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let clients_clone = clients.clone();
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
                        let mut client_stream = client_stream_mutex.lock().unwrap();
                        let _ = client_stream.write_all(join_msg.as_bytes());
                        let _ = client_stream.write_all(b"\n");
                    }
                }
                ServerMessage::ClientDisconnected(username) => {
                    println!("Client {username} disconnected.");
                    clients_clone.lock().unwrap().remove(&username);
                    let disconnected_msg = format!("{username} has left chat.");
                    for (_, client_stream_mutex) in clients_clone.lock().unwrap().iter() {
                        let mut client_stream = client_stream_mutex.lock().unwrap();
                        let _ = client_stream.write_all(disconnected_msg.as_bytes());
                        let _ = client_stream.write_all(b"\n");
                    }
                }
                ServerMessage::ChatMessage(sender, content) => {
                    let full_message = format!("[{sender}]: {content}");
                    println!("Broadcasting: {}", full_message.trim());
                    for (name, client_stream_mutex) in clients_clone.lock().unwrap().iter() {
                        if name == &sender {
                            continue;
                        }
                        let mut client_stream = client_stream_mutex.lock().unwrap();
                        let _ = client_stream.write_all(full_message.as_bytes());
                        let _ = client_stream.write_all(b"\n");
                    }
                }
            }
        }
    });

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let tx_clone = tx_server.clone();
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
                            username = msg.trim().to_string();
                            let _ = tx_clone.send(ServerMessage::NewClient(
                                username.clone(),
                                arc_stream.clone(),
                            ));
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
                                let _ = tx_clone.send(ServerMessage::ChatMessage(
                                    username.clone(),
                                    message_content.trim().to_string(),
                                ));
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
