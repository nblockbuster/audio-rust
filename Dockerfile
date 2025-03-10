FROM rust:1.85-bullseye as build

WORKDIR /audio-bot

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src

RUN apt update && apt -y install cmake && apt -y install pkg-config

RUN cargo build --release

CMD ["./target/release/audio-bot"]
