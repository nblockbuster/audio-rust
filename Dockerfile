FROM rust:1.85-alpine AS build

WORKDIR /audio-bot

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src

RUN apk add git make g++ gcc cmake pkgconf musl-dev

RUN cargo build --release

FROM alpine:3.21

COPY --from=build /audio-bot/target/release/audio-bot /

RUN apk add --no-cache yt-dlp

CMD ["./audio-bot"]