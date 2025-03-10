FROM rust:1.85-bullseye as build

RUN USER=root cargo new --bin audio-bot
WORKDIR /audio-bot

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src

RUN apt update && apt -y install cmake && apt -y install pkg-config

RUN cargo build --release

FROM rust:1.85-slim

COPY --from=build /audio-bot/target/release/audio-bot .
CMD ["./audio-bot"]
