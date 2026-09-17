# Featherboard

A self-hosted feedback board: submit ideas, vote, discuss, and track them on a public roadmap and changelog.

## Configuration

Featherboard is configured via environment variables.

| Variable | Required | Default | Description                                                                                                        |
|---|---|---|--------------------------------------------------------------------------------------------------------------------|
| `DATABASE_URL` | No | `sqlite:featherboard.db?mode=rwc` | SQLite database connection string.                                                                                 |
| `PORT` | No | `3000` | Port the server listens on.                                                                                        |
| `PUBLIC_URL` | Only for email notifications | — | Public base URL used to build links in notification emails. |
| `SMTP_ADDRESS` | Only for email notifications | — | SMTP server host used to send admin notification emails (new comments, status changes, changelog publishes).       |
| `SMTP_PORT` | Only for email notifications | — | SMTP server port.                                                                                                  |
| `SMTP_USER` | Only for email notifications | — | SMTP auth username.                                                                                                |
| `SMTP_PASSWORD` | Only for email notifications | — | SMTP auth password.                                                                                                |

Note: GitHub OAuth login is **not** configured via environment variables — each organization registers its own GitHub OAuth app, with the client ID/secret stored per-organization in the database rather than as a global env var.
