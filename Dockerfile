FROM rust:1.85 as build

RUN USER=root cargo new --bin audio-bot
WORKDIR /audio-bot

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release
RUN rm src/*.rs

COPY ./src ./src
RUN rm ./target/release/deps/audio-bot*
RUN cargo build --release


FROM rust:1.85-slim

COPY --from=build /audio-bot/target/release/audio-bot .
CMD ["./audio-bot"]
