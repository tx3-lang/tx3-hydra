# Tx3-hydra

Tx3-hydra provides support for Hydra state channels within the Tx3 ecosystem. It acts as a Transaction Resolution Protocol (TRP) server, allowing clients to resolve and submit transactions to a connected Hydra Head.

## Features

- Connects to a Hydra Head via WebSocket.
- Implements the TRP for transaction resolution and submission.
- Provides a JSON-RPC interface for client interaction.
- Configurable via `config.toml` or environment variables.

## Prerequisites

- Rust and Cargo (latest stable version recommended)
- A running Hydra Head instance

## Building and Running

1.  Clone the repository:
    ```sh
    git clone https://github.com/tx3-lang/tx3-hydra.git
    cd tx3-hydra
    ```
2.  Create a `config.toml` file in the project root or set the `TRP_HYDRA_CONFIG` environment variable pointing to your config file. See `examples/basic/config.toml` for an example.
3.  Build and run the project:
    ```sh
    cargo run
    ```
    The TRP server will start and listen on the configured address.

## Configuration

The project can be configured using a `config.toml` file or environment variables prefixed with `TRP_HYDRA_`.

Example `config.toml`:

```toml
[trp]
listen_address = "0.0.0.0:8164"
permissive_cors = false
max_optimize_rounds = 10

[hydra]
network = 0 # Cardano network ID (e.g., 0 for Testnet, 1 for Mainnet)
ws_url = "ws://127.0.0.1:4001" # WebSocket URL of the Hydra Head
http_url = "http://127.0.0.1:4001" # HTTP URL of the Hydra Head (for fetching parameters)
```

## TRP Interface

The TRP server exposes the following JSON-RPC methods:

-   `trp.resolve`: Resolves a Tx3 transaction.
-   `trp.submit`: Submits a resolved and signed transaction to the Hydra Head.
-   `health`: Checks the health of the TRP server and its connection to the Hydra Head.

See the [Basic Example](examples/basic/README.md) for detailed examples on how to use these methods with `curl`.

## Examples

-   [Basic Example](examples/basic/README.md): Demonstrates how to run tx3-hydra and interact with its TRP interface.

## License

This project is licensed under the Apache 2.0 License. See the [LICENSE](LICENSE) file for details.
