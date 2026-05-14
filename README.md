# Prompt Eval

Minimal prompt evaluation app with:

- Rust API (`axum` + `sqlx` + Postgres)
- Next.js UI (`prompt-eval-ui`)

## Requirements

- Rust toolchain
- Bun
- PostgreSQL

## Environment

Create `.env` in the repo root (you can copy from `.env.example`):

```env
ANTHROPIC_API_KEY=your_key_here
ANTHROPIC_MODEL=claude-haiku-4-5-20251001
DATABASE_URL=postgresql://postgres:password@localhost:5432/prompt_eval
```

## Run

Backend (from repo root):

```bash
cargo run
```

Frontend (new terminal):

```bash
cd prompt-eval-ui
bun install
bun run dev
```

## App URLs

- UI: `http://localhost:3000`
- API: `http://127.0.0.1:3001/api`

## Key API Routes

- `GET /api/prompts`
- `GET /api/prompts/:id`
- `POST /api/prompts/generate`
- `POST /api/questions/generate`
- `GET /api/datasets`
- `POST /api/evaluate`

image.png
