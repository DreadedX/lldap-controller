FROM rust:1.92 AS base
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
RUN cargo install cargo-chef --locked --version 0.1.73 && \
    cargo install cargo-auditable --locked --version 0.7.2
WORKDIR /app

FROM base AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM base AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
ARG RELEASE_VERSION
ENV RELEASE_VERSION=${RELEASE_VERSION}
RUN cargo auditable build --release && /app/target/release/crdgen > /crds.yaml

FROM scratch AS manifests
COPY --from=builder /crds.yaml /

FROM gcr.io/distroless/cc-debian13:nonroot AS runtime
COPY --from=builder /app/target/release/lldap-controller /lldap-controller
CMD ["/lldap-controller"]
