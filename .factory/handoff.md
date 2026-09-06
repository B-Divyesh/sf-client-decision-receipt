# Handoff — Client Decision Receipt

## Release status

Implementation commit: `5b0d32bc54c8d8a60d2bd632d8c6d383eaceca55`.

Client Decision Receipt gives freelancers and small studios a private link for
an explicit accept, changes request, or decline. It freezes the shown scope in
a dated SHA-256 receipt. The first action is **Try it with sample data**.

The final image is live at `https://client-decision-receipt.sociobot.in` on
one healthy replica with the `/data` Azure Files mount. Its `/health` endpoint
returns implementation SHA `5b0d32bc54c83b9bf3be26ce12f46e4788e2c4aa`.

## What changed

- Added `/demo`: realistic Fern Studio sample data in a separate 24-hour
  workspace, persistent **Demo — sample data, nothing is saved** label, reset,
  and start-for-real actions. Demo data never uses proposal or mail records.
- Added `.factory/claims.json` with outcome browser tests for demo isolation,
  frozen receipts, no tracking, offline reload, and the free core.
- Made private-link state durable. The service is pinned to one replica. Azure
  Files byte-range locks do not support a live SQLite file, so the app restores
  local SQLite from `/data/receipts-durable.sqlite` and writes a fully
  checkpointed SQLite snapshot to `/data` after every real-data write. A
  restart regression test proves a private link survives this handoff.
- Losing simultaneous decisions return `409`; an integration test asserts one
  `201` and one `409`.
- All APIs are rate limited. The final deployment fixes `Retry-After` to one
  usable second for both read and write limits.
- Added designed 404 behaviour, titles, skip-link focus, service-worker update
  action, sitemap, social metadata, accessibility checks, copy audit, catalog
  description, and generated-image provenance.
- When no SMTP sender exists, delivery says `sender_not_configured` instead of
  pretending an email has been queued for sending.
- Kept the public $29 one-time Independent Pro offer and restore path. The
  live Sociobot checkout returned 404, so checkout is visibly unavailable
  rather than a dead purchase button. Billing metadata is at
  `/work/.evidence/billing-offer.json`.

## Verification

From a clean checkout:

```bash
npm ci
npm test
npm run test:claims
npm run build
cargo clippy --all-targets -- -D warnings
cargo build --release --locked
```

Completed during this repair:

- `npm test` passed: Vitest, 7 Rust tests, and 8 Playwright tests.
- Every documented claim command passed from the demo sandbox.
- `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`,
  `npm run build`, and zero-config runtime `/health` passed.
- Built frontend: JavaScript 78.81 KB (28.60 KB gzip), CSS 16.92 KB
  (4.69 KB gzip).
- Playwright axe checks found no serious or critical violations on home or
  demo; mobile width had no overflow.
- Live on `0c4efc9`: health returned the exact build SHA; eight repeated reads
  of one private link all returned 200; concurrent decisions returned 201 and
  409; the management receipt returned 200 and was then deleted. Fresh desktop
  and phone contexts showed the job headline, audience, and sample action
  before scrolling. The phone demo showed the sample, persistent label, and
  reset with no console errors.
- `verify-url.sh` passed against the live page: title, language, h1, main,
  alt checks, and no console errors.
- Final live `5b0d32b` check: one healthy, active replica; `/health` returned
  its exact build SHA; the read allowance returned 429 with `Retry-After: 1`.

## Known external dependencies

- The factory supplies only `PORT`. No SMTP sender credentials were available
  in product scope, so the required two-party emails cannot be sent by default.
  Configure a trusted STARTTLS sender with the documented SMTP variables, then
  perform an inbox/deliverability check. Until then, the UI honestly reports
  `sender not configured`.
- Sociobot billing registration is still required. On 2026-09-06 the checkout
  URL returned 404. The free core is unaffected; do not enable purchase until
  the billing operator registers the stated offer.

## Operations

No variable other than `PORT` is required. The durable secret and SQLite
snapshot live under `/data`; keep the deployment at one replica.

```bash
docker build --build-arg BUILD_SHA=<source-commit> -t client-decision-receipt .
docker run --rm -p 8080:8080 -v cdr-data:/data client-decision-receipt
```
