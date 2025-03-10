FROM rust:1.85-bullseye as build

WORKDIR /audio-bot

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src

RUN apt-get update && apt-get -y install software-properties-common
RUN add-apt-repository ppa:tomtomtom/yt-dlp
RUN apt-get update && apt-get -y install cmake && apt-get -y install pkg-config && apt-get -y install yt-dlp

RUN cargo build --release

FROM rust:1.85-slim

COPY --from=build /audio-bot/target/release/audio-bot .

RUN apt-get update && apt-get -y install software-properties-common
RUN add-apt-repository ppa:tomtomtom/yt-dlp
RUN apt-get update && apt-get -y install yt-dlp

CMD ["./audio-bot"]