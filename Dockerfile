FROM rust:1.80.0-alpine3.20 AS builder
WORKDIR /usr/src/flight
RUN apk add --no-cache git musl-dev python3-dev
COPY . .
ARG RUSTFLAGS=-Ctarget-feature=-crt-static
RUN cargo install --path .

FROM alpine:3.20
RUN apk add --no-cache python3
COPY --from=builder /usr/local/cargo/bin/flight /usr/local/bin/flight
CMD ["flight"]