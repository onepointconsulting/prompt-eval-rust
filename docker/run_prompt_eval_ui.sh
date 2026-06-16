#!/bin/sh
set -e

# Run the UI only. FRPC is handled by nginx container.
exec bun /app/server.js