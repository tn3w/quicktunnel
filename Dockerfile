FROM rust:alpine AS builder

WORKDIR /build

RUN apk add --no-cache musl-dev

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --target x86_64-unknown-linux-musl

FROM scratch

COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/quicktunnel /quicktunnel

EXPOSE 3000 8080 22

ENTRYPOINT ["/quicktunnel"]
