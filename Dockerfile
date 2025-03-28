FROM rust:1.85-bookworm AS build

WORKDIR /audio-bot

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src

RUN apt-get update && apt-get -y install cmake && apt-get -y install pkg-config

RUN cargo build --release

FROM rust:1.85-slim-bookworm

COPY --from=build /audio-bot/target/release/audio-bot .

ENV PATH="/root/.local/bin:$PATH" 
RUN apt-get update && apt-get -y install pipx
RUN pipx install yt-dlp

CMD ["./audio-bot"]