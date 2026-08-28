# Client Decision Receipt

A privacy-first decision utility for freelancers. Create a private proposal link, let a client explicitly **Accept**, **Request changes**, or **Decline**, then keep the exact scope, date, response, and SHA-256 seal as a portable receipt.

It is an acknowledgement and operational record—not a regulated electronic-signature service. There are no tracking pixels, open counters, advertising scripts, or third-party runtime assets.

## Who it is for

Independent freelancers and tiny studios that need an unambiguous next step without adopting a full proposal or e-signature suite.

## What v1 includes

- Account-free creation using separate unguessable client and management links
- One immutable client decision per proposal
- Frozen line-item scope, amount, UTC timestamps, and canonical SHA-256 receipt seal
- Private management view, browser-local archive, JSON/CSV export, print/PDF, and confirmed deletion
- Durable two-recipient email queue; automatic delivery when SMTP is configured
- $29 one-time Independent Pro unlock through Sociobot billing, with daily verification and license restore
- Responsive light/dark botanical field-guide interface, offline shell, legal pages, and reduced-motion support

## Develop and test

Requirements: Node 22+, Rust 1.85+.

```bash
npm ci
npm run build            # production frontend -> dist/
DATA_DIR=./data cargo run # API + built frontend on :8080
npm test                 # Vitest + Rust unit/integration tests
```

For frontend hot reload, run `cargo run` with `DATA_DIR=./data` and `npm run dev` in separate terminals; Vite proxies `/api` and `/health` to port 8080.

## Configuration

No environment variables are required. `PORT` defaults to `8080`; the SQLite database and generated instance secret default to `/data`.

Optional variables:

- `DATA_DIR` or `DATABASE_URL` — persistence location override
- `INSTANCE_SECRET` — token-HMAC secret override (otherwise generated securely on first boot)
- `SMTP_HOST`, `SMTP_PORT`, `SMTP_FROM`, and optionally `SMTP_USERNAME`/`SMTP_PASSWORD` — STARTTLS receipt delivery. Without SMTP, both messages stay visible as `queued` in the durable SQLite outbox.
- `DIST_DIR` — built frontend directory (defaults to `dist`)

## Container

```bash
docker build --build-arg BUILD_SHA="$(git rev-parse HEAD)" -t client-decision-receipt .
docker run --rm -p 8080:8080 -v cdr-data:/data client-decision-receipt
curl http://localhost:8080/health
```

The image is multi-stage, runs as non-root UID 10001, exposes port 8080, and accepts the factory `BUILD_SHA` build argument without requiring `.git`.

## Security and data

Client and management capabilities are separate 192-bit random tokens. Database lookups use a persisted HMAC key. Inputs are validated server-side and parameterized through SQLx. API reads allow a burst of 40 and replenish at 20 requests/second; writes allow a burst of 6 and replenish at 1 request/second. Both use the first `X-Forwarded-For` hop and return `429` with `Retry-After`. Deletion removes personal data while retaining only a receipt hash tombstone, catalogue number, and dates.

See [privacy](https://client-decision-receipt.sociobot.in/privacy), [terms](https://client-decision-receipt.sociobot.in/terms), and [the visual thesis](.factory/design.md).

## License

MIT — see [LICENSE](LICENSE).
