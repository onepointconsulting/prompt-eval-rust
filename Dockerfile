FROM rust:1-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY .sqlx ./.sqlx
COPY docs/queries/db-queries.sql ./docs/queries/db-queries.sql

ENV SQLX_OFFLINE=true
RUN cargo build --release

# 1. Prepare frpc
RUN mkdir -p /frpc/
COPY docker/run_frpc.sh /frpc/
ADD https://github.com/fatedier/frp/releases/download/v0.65.0/frp_0.65.0_linux_amd64.tar.gz /tmp/frp.tar.gz
RUN tar -xzf /tmp/frp.tar.gz -C /tmp/ && \
    mv /tmp/frp_0.65.0_linux_amd64/frpc /frpc/frpc && \
    chmod +x /frpc/frpc /frpc/run_frpc.sh && \
    rm -rf /tmp/frp.tar.gz /tmp/frp_0.65.0_linux_amd64

FROM debian:bookworm-slim AS runner

COPY docker/run_prompt_eval_rust.sh /docker/run.sh
RUN chmod +x /docker/run.sh

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/prompt_eval /usr/local/bin/prompt_eval
COPY --from=builder /app/docs/queries/db-queries.sql /var/sql/db-queries.sql

# Copy frpc from builder stage
COPY --from=builder /frpc /frpc
RUN chmod +x /frpc/frpc /frpc/run_frpc.sh

ENV BOOTSTRAP_SCRIPT=/var/sql/db-queries.sql

EXPOSE 3001

CMD ["/docker/run.sh"]
