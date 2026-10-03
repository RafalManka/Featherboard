# Featherboard

A self-hosted feedback board: submit ideas, vote, discuss, and track them on a public roadmap and changelog.

## Configuration

Featherboard is configured via environment variables.

| Variable | Required | Default | Description                                                                                                        |
|---|---|---|--------------------------------------------------------------------------------------------------------------------|
| `DATABASE_URL` | No | `sqlite:featherboard.db?mode=rwc` | SQLite database connection string.                                                                                 |
| `PORT` | No | `3000` | Port the server listens on.                                                                                        |
| `PUBLIC_URL` | Only for email notifications and Stripe Checkout | — | Public base URL used to build notification links and Stripe Checkout return URLs. |
| `SMTP_ADDRESS` | Only for email notifications | — | SMTP server host used to send admin notification emails (new comments, status changes, changelog publishes).       |
| `SMTP_PORT` | Only for email notifications | — | SMTP server port.                                                                                                  |
| `SMTP_USER` | Only for email notifications | — | SMTP auth username.                                                                                                |
| `SMTP_PASSWORD` | Only for email notifications | — | SMTP auth password.                                                                                                |
| `STRIPE_SECRET_KEY` | Only for Stripe Checkout | — | Stripe secret API key used to create Checkout Sessions.                                                           |
| `STRIPE_PRICE_ID` | Only for Stripe Checkout | — | Recurring Stripe Price ID used for the hosted subscription plan.                                                  |
| `STRIPE_WEBHOOK_SECRET` | Only for Stripe webhooks | — | Stripe webhook signing secret used to verify incoming events.                                                     |

Note: GitHub OAuth login is **not** configured via environment variables — each organization registers its own GitHub OAuth app, with the client ID/secret stored per-organization in the database rather than as a global env var.
