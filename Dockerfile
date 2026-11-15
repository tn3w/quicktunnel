FROM rust:alpine AS build
WORKDIR /build
COPY . .
RUN cargo build --release

FROM scratch
COPY --from=build /build/target/release/quicktunnel /quicktunnel
WORKDIR /data
ENTRYPOINT ["/quicktunnel"]
