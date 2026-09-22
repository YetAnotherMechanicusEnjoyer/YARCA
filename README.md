# Yet Another Rust Chat App

## Table of Content

- [About](#about)
- [Installation](#installation)
  * [Dependencies](#dependencies)
  * [Crates](#Crates)
  * [Server](#server)
  * [Client](#client)
- [Usage](#usage)
  * [Server](#server-1)
  * [Client](#client-1)
- [License](#license)

## About

> [!NOTE]
> A Chat Terminal App that connects to a server via IP Address and communicates with TCP Streams.

## Installation

### Dependencies

> [!IMPORTANT]
> Make sure to have [Rust](https://www.rust-lang.org/tools/install) installed.

### Crates
- aes-gcm
 - version: 0.10.3
 - features: aes
- crossterm
 - version: 0.29.0
- dotenvy
 - version: 0.15.7
- hex
 - version: 0.4.3
- rand
 - version: 0.9.2
- you
 - version: happy

### Server

> [!NOTE]
> Clone the repo somewhere, make a `.env` file at the root of the repository that contains a 32 bits-long secret variable and an address.

> [!TIP]
> Exemple of a `.env` file :
```env
ADDR="127.0.0.1:8080"
SECRET="your-32-bits-long-variable-here!"
```

> [!NOTE]
> Compile the server binary with [Cargo](https://doc.rust-lang.org/cargo/).

```bash
cargo build --release --bin server
```

### Client

> [!NOTE]
> Clone the repo somewhere and compile the client binary with [Cargo](https://doc.rust-lang.org/cargo/).

```bash
cargo build --release --bin client
```

## Usage

### Server

> [!NOTE]
> Start the server with [Cargo](https://doc.rust-lang.org/cargo/) after building it, or execute the binary.

Example :
```bash
cargo run --release --bin server
```


### Client

> [!NOTE]
> Run client with [Cargo](https://doc.rust-lang.org/cargo/) after building it, or execute the binary.

Example :
```bash
cargo run --release --bin client
```


## Licence
[MIT](https://github.com/YetAnotherMechanicusEnjoyer/YARCA/blob/53174069377b73f1c96ca9761ef2c6ec93532167/LICENSE)
