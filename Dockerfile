FROM rust:alpine AS builder

WORKDIR /build

RUN apk add --no-cache musl-dev

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY index.html ./index.html

RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:latest

RUN apk add --no-cache ca-certificates

WORKDIR /app

COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/quicktunnel /app/quicktunnel
COPY --from=builder /build/index.html /app/index.html

RUN mkdir -p /app/keys

EXPOSE 3000 8080 22

CMD ["/app/quicktunnel"]
