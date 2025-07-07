# Vending Machine Example

This project demonstrates a vending machine interacting with a Hydra Head via a TX3 layer.

## Prerequisites

- Docker and Docker Compose installed.

## How to Run

1.  Navigate to the root directory of this example:
    ```bash
    cd examples/vending-machine
    ```
2.  Build and run the Docker containers using Docker Compose:
    ```bash
    docker-compose up --build
    ```
    This will start the `hydra-node`, `hydra-tx3`, and `vending-machine` services.

## Accessing the Vending Machine

Once the Docker containers are running, you can access the vending machine web interface by opening your web browser and navigating to:

```
http://localhost:8080
```
