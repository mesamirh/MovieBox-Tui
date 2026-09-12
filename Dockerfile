FROM rust:1.90-bookworm AS builder

WORKDIR /app
COPY . .
RUN cargo build --release --bin api

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/api /usr/local/bin/moviebox-api

ENV PORT=7860
EXPOSE 7860

CMD ["moviebox-api"]
