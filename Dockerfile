FROM rust:1.85-alpine as build

WORKDIR /audio-bot

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src

RUN apk update && apk add musl-dev && apk add openssl-dev && apk add cmake && apk add pkgconfig && apk add yt-dlp

RUN cargo build --release

CMD ["./target/release/audio-bot"]
