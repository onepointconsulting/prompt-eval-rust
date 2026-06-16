FROM rust:1-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY .sqlx ./.sqlx
COPY docs/queries/db-queries.sql ./docs/queries/db-queries.sql

ENV SQLX_OFFLINE=true
RUN cargo build --release

FROM debian:bookworm-slim AS runner

COPY docker/run_prompt_eval_rust.sh /docker/run.sh
RUN sed -i 's/\r$//' /docker/run.sh && chmod +x /docker/run.sh

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/prompt_eval /usr/local/bin/prompt_eval
COPY --from=builder /app/docs/queries/db-queries.sql /var/sql/db-queries.sql

ENV BOOTSTRAP_SCRIPT=/var/sql/db-queries.sql

EXPOSE 3001

CMD ["/docker/run.sh"]
