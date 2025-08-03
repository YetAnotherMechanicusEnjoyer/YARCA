# Yet Another Rust Chat App

## Table of Content

- [About](#about)
- [Installation](#installation)
  * [Dependencies](#dependencies)
- [Usage](#usage)
  * [Server](#server)
  * [Client](#client)
- [License](#license)

## About

> [!NOTE]
> A Chat Terminal App that connects to a server via IP Address and communicates with TCP Streams.

## Installation

### Dependencies

> [!IMPORTANT]
> Make sure to have [Rust](https://www.rust-lang.org/tools/install) installed.

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
cargo build --release --bin YARCA
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

Cargo :
```bash
cargo run --release --bin YARCA
```

Binary :
```bash
chmod +x /path/to/repo/target/release/YARCA
/path/to/repo/target/release/YARCA
```

### Client

> [!NOTE]
> Run client with [Cargo](https://doc.rust-lang.org/cargo/) after building it, or execute the binary.

Cargo :
```bash
cargo run --release --bin client
```

Binary :
```bash
chmod +x /path/to/repo/target/release/client
/path/to/repo/target/release/client
```

## Licence
[MIT](https://github.com/YetAnotherMechanicusEnjoyer/YARCA/blob/53174069377b73f1c96ca9761ef2c6ec93532167/LICENSE)
