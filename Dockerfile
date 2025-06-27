FROM rust:1.87.0-slim-bullseye AS build

WORKDIR /app

RUN apt update
RUN apt install -y build-essential pkg-config libssl-dev libsasl2-dev cmake

COPY ./Cargo.toml ./Cargo.toml
COPY . .

RUN cargo build --release

FROM debian:stable-slim

COPY --from=build /app/target/release/tx3-hydra /usr/local/bin/tx3-hydra

ENTRYPOINT [ "tx3-hydra" ]
