FROM rust:1.90-bookworm AS builder

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
COPY static ./static
COPY templates ./templates

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install --no-install-recommends -y ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home --home-dir /data featherboard

WORKDIR /data

COPY --from=builder /build/target/release/featherboard /usr/local/bin/featherboard

RUN chown featherboard:featherboard /data

USER featherboard

EXPOSE 3000

ENV DATABASE_URL=sqlite:/data/featherboard.db?mode=rwc \
    PORT=3000

ENTRYPOINT ["/usr/local/bin/featherboard"]
