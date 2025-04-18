FROM clux/muslrust:stable AS builder
ARG TARGETARCH

WORKDIR /usr/src/app
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src

RUN cargo build --release
RUN ls -lh target

FROM alpine

COPY --from=builder /usr/src/app/target/*-unknown-linux-musl/release/rust-mcp-filesystem rust-mcp-filesystem

ENTRYPOINT ["./rust-mcp-filesystem"]

