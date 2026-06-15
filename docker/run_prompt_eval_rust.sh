#!/bin/sh
set -e

# Run the API only. FRPC is handled by nginx container.
exec /usr/local/bin/prompt_eval