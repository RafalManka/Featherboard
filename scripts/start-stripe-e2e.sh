#!/usr/bin/env bash

set -Eeuo pipefail

port="${STRIPE_E2E_PORT:-3103}"
database_file="${STRIPE_E2E_DATABASE_FILE:-/private/tmp/featherboard-stripe-e2e.db}"
listen_log="$(mktemp /private/tmp/featherboard-stripe-listen.XXXXXX)"
app_pid=""
stripe_pid=""

cleanup() {
  local exit_code=$?

  if [[ -n "$app_pid" ]] && kill -0 "$app_pid" 2>/dev/null; then
    kill "$app_pid" 2>/dev/null || true
  fi
  if [[ -n "$stripe_pid" ]] && kill -0 "$stripe_pid" 2>/dev/null; then
    kill "$stripe_pid" 2>/dev/null || true
  fi

  wait "$app_pid" 2>/dev/null || true
  wait "$stripe_pid" 2>/dev/null || true
  rm -f "$listen_log"
  exit "$exit_code"
}

trap cleanup EXIT INT TERM

if ! command -v stripe >/dev/null 2>&1; then
  echo 'Stripe CLI is required. Install it from https://docs.stripe.com/stripe-cli.' >&2
  exit 1
fi

for variable in STRIPE_SECRET_KEY STRIPE_PRICE_ID; do
  if [[ -z "${!variable:-}" ]]; then
    echo "${variable} must be set for Stripe E2E tests." >&2
    exit 1
  fi
done

rm -f "$database_file" "$database_file-shm" "$database_file-wal"

stripe listen \
  --events checkout.session.completed,checkout.session.expired,customer.subscription.created,customer.subscription.updated,customer.subscription.deleted,invoice.paid,invoice.payment_failed \
  --forward-to "http://127.0.0.1:${port}/webhooks/stripe" \
  >"$listen_log" 2>&1 &
stripe_pid=$!

webhook_secret=''
for _ in {1..60}; do
  webhook_secret="$(sed -nE 's/.*(whsec_[A-Za-z0-9]+).*/\1/p' "$listen_log" | head -n 1)"
  if [[ -n "$webhook_secret" ]]; then
    break
  fi
  sleep 0.5
done

if [[ -z "$webhook_secret" ]]; then
  echo 'Stripe CLI did not provide a webhook signing secret:' >&2
  sed -n '1,120p' "$listen_log" >&2
  exit 1
fi

E2E_PORT="$port" \
E2E_DATABASE_FILE="$database_file" \
PUBLIC_URL="http://127.0.0.1:${port}" \
STRIPE_WEBHOOK_SECRET="$webhook_secret" \
cargo run --features e2e --bin e2e-server &
app_pid=$!

wait "$app_pid"
