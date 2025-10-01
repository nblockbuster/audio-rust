FROM lukemathwalker/cargo-chef:latest-rust-1.85.1-alpine3.20 AS chef
RUN apk add git cmake make musl-dev pkgconf
ENV SCCACHE_VERSION=0.10.0
RUN wget -O sccache.tar.gz https://github.com/mozilla/sccache/releases/download/v${SCCACHE_VERSION}/sccache-v${SCCACHE_VERSION}-x86_64-unknown-linux-musl.tar.gz && \
  tar xzf sccache.tar.gz && \
  mv sccache-v*/sccache /usr/local/bin/sccache && \
  chmod +x /usr/local/bin/sccache
ENV RUSTC_WRAPPER=/usr/local/bin/sccache SCCACHE_DIR=/sccache
# RUN cargo install cargo-chef

FROM chef AS planner
WORKDIR /app
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src
RUN --mount=type=cache,target=$SCCACHE_DIR,sharing=locked cargo chef prepare --recipe-path recipe.json

FROM chef AS build
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=$SCCACHE_DIR,sharing=locked cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN --mount=type=cache,target=$SCCACHE_DIR,sharing=locked cargo build --release

FROM alpine:3.21 AS runtime
COPY --from=build /app/target/release/audio-bot /
RUN apk add --no-cache yt-dlp
RUN apk update
RUN apk upgrade
CMD ["./audio-bot"]
