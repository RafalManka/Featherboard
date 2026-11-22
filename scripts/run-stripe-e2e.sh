#!/usr/bin/env bash

set -Eeuo pipefail

credentials_file=".env.test.local"
if [[ ! -f "$credentials_file" ]]; then
  echo "Missing $credentials_file. Copy .env.test.local.example and fill in the Stripe test values." >&2
  exit 1
fi

set -a
source "$credentials_file"
set +a

exec env E2E_PROJECT=stripe STRIPE_E2E=1 playwright test "$@"
