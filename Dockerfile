FROM rust:1.85-slim-bullseye as build

WORKDIR /audio-bot

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src

RUN add-apt-repository ppa:tomtomtom/yt-dlp
RUN apt update && apt -y install cmake && apt -y install pkg-config && apt -y install yt-dlp

RUN cargo build --release

CMD ["./target/release/audio-bot"]
