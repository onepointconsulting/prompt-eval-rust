FROM oven/bun:1 AS builder

WORKDIR /app

# 1. Prepare frpc
RUN mkdir -p /frpc/
COPY docker/run_frpc.sh /frpc/
ADD https://github.com/fatedier/frp/releases/download/v0.65.0/frp_0.65.0_linux_amd64.tar.gz /tmp/frp.tar.gz
RUN tar -xzf /tmp/frp.tar.gz -C /tmp/ && \
    mv /tmp/frp_0.65.0_linux_amd64/frpc /frpc/frpc && \
    chmod +x /frpc/frpc /frpc/run_frpc.sh && \
    rm -rf /tmp/frp.tar.gz /tmp/frp_0.65.0_linux_amd64

COPY prompt-eval-ui/package.json prompt-eval-ui/bun.lock ./
RUN bun install --frozen-lockfile

COPY prompt-eval-ui/ .

ARG NEXT_PUBLIC_API_BASE_URL=http://localhost:3001/api
ENV NEXT_PUBLIC_API_BASE_URL=$NEXT_PUBLIC_API_BASE_URL

RUN bun run build

FROM oven/bun:1 AS runner

COPY docker/run_prompt_eval_ui.sh /docker/run.sh
RUN chmod +x /docker/run.sh

# Copy frpc from builder stage
COPY --from=builder /frpc /frpc
RUN chmod +x /frpc/frpc /frpc/run_frpc.sh

WORKDIR /app

ENV NODE_ENV=production
ENV PORT=3000
ENV HOSTNAME=0.0.0.0

COPY --from=builder /app/.next/standalone ./
COPY --from=builder /app/.next/static ./.next/static
COPY --from=builder /app/public ./public

EXPOSE 3000

CMD ["/docker/run.sh"]
