FROM oven/bun:1 AS builder

WORKDIR /app

COPY prompt-eval-ui/package.json prompt-eval-ui/bun.lock ./
RUN bun install --frozen-lockfile

COPY prompt-eval-ui/ .

ARG NEXT_PUBLIC_API_BASE_URL=http://prompt-backend.onepointltd.ai/api
ENV NEXT_PUBLIC_API_BASE_URL=$NEXT_PUBLIC_API_BASE_URL

RUN bun run build

FROM oven/bun:1 AS runner

COPY docker/run_prompt_eval_ui.sh /docker/run.sh
RUN sed -i 's/\r$//' /docker/run.sh && chmod +x /docker/run.sh

WORKDIR /app

ENV NODE_ENV=production
ENV PORT=3000
ENV HOSTNAME=0.0.0.0

COPY --from=builder /app/.next/standalone ./
COPY --from=builder /app/.next/static ./.next/static
COPY --from=builder /app/public ./public

EXPOSE 3000

CMD ["/docker/run.sh"]
