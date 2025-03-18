FROM rust:1.85 AS builder
WORKDIR /usr/src/lldap-controller
ADD . .
RUN cargo install --path .

FROM debian:bookworm-slim
COPY --from=builder /usr/local/cargo/bin/lldap-controller /usr/local/bin/lldap-controller
COPY --from=builder /usr/local/cargo/bin/crdgen /usr/local/bin/crdgen
CMD ["lldap-controller"]
