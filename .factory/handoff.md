# Handoff — Client Decision Receipt

## Independent verification outcome (2026-08-28): **FAIL**

Candidate `75019d3cb52948843ff2f2db890cf01a2c13342b` was independently checked
against https://client-decision-receipt.sociobot.in from a clean checkout. Do
not release or promote it. The detailed evidence is in
[`verification.md`](verification.md).

- **P0:** live capability links alternate between `404` and `200` on repeated
  requests to the same token, proving per-instance database/HMAC-secret state
  behind the load balancer.
- **P1:** `.factory/claims.json` is missing; mandatory claim tests could not be
  run. The home page has no one-click sample-data demo, a separately mandated
  FAIL condition.
- **P1:** default factory runtime has no configured SMTP, so the brief-required
  two-party emails remain `queued`; concurrent final decisions return `500` for
  the losing request instead of `409`.

The prior builder verification below is retained as implementation context, but
it is superseded by this release decision and does not cover the live
multi-instance persistence failure.

## Shipped

- Rust/Axum service with SQLite persistence, structured logs, graceful shutdown, `/health` build identity, secure headers, compression, and static frontend serving on `PORT` (default 8080).
- Account-free proposal creation with independent 192-bit client and management capabilities; lookup values are HMAC-protected by a CSPRNG secret generated and persisted at first boot.
- Explicit Accept / Request changes / Decline workflow. The first response wins transactionally; proposal metadata, exact line items, party details, response, and UTC timestamps are canonicalized into an immutable SHA-256 receipt seal.
- Management view with status, client-link recovery, delivery status, print/PDF, JSON and CSV exports, and confirmed deletion. Deletion preserves only the anonymous hash/catalogue/date tombstone.
- Durable receipt messages for both supplied email addresses. With optional STARTTLS SMTP configuration they send asynchronously and retry queued work on restart; without SMTP they remain visibly queued rather than being lost or claimed as sent.
- Free core plus the Sociobot `$29` one-time Pro contract: hosted checkout, URL-token capture and stripping, local storage, daily verification cache, optimistic offline behavior, revoked-license notice, paste-to-restore, archive-index export, and priority support. Core exports, deletion, and accessibility remain free.
- Original botanical field-guide system, responsive 390 px treatment, dark theme, reduced-motion fallback, empty/loading/error/offline states, keyboard-native forms, legal pages, and offline shell.
- Original factory-generated herbarium illustration with prompt/model provenance. Runtime WebP sources are 22 KB (640 px) and 53 KB (960 px).

## Run and deploy

```bash
npm ci
npm test
npm run build
DATA_DIR=./data cargo run
```

Container build: `docker build --build-arg BUILD_SHA=<source-commit> -t client-decision-receipt .`

No runtime variable is required. The image defaults to port 8080 and creates `/data/receipts.sqlite` plus `/data/instance-secret`; mount `/data` persistently. See `README.md` for optional SMTP variables.

## Verification performed

- `npm audit --audit-level=high`: **0 vulnerabilities**.
- `npm test`: **passed** — 2 Vitest assertions, 3 Rust tests, and 2 Playwright journeys. The browser test runs at 390×844, creates a proposal, records the decision, checks the seal, exports JSON, deletes the record, and asserts no console errors.
- Playwright + axe-core on the decided receipt: **0 serious or critical violations**.
- `cargo clippy --all-targets -- -D warnings`: passed after formatting.
- `npm run build`: passed; output is exactly `dist/`. Initial JS 71.87 KB / 26.98 KB gzip; CSS 15.71 KB / 4.44 KB gzip.
- Lighthouse mobile: **Performance 98, Accessibility 100, Best Practices 100, SEO 100**. LCP 1.5 s, CLS 0, total blocking time 140 ms.
- `cargo build --release --locked`: passed.
- Zero-config runtime smoke: release binary launched with a cleared environment plus `PORT`; `/health` returned `{status: ok, buildSha: dev}`.
- Load smoke: 100 concurrent `/health` requests completed in 163 ms with 100/100 HTTP 200 responses. API burst test separately verifies HTTP 429 and `Retry-After`; health is intentionally exempt.
- Manual review at 390×844: no horizontal page overflow and all controls remain at least 44 px.
- Asset review: coherent fern, blank tag, correct palette, no malformed text, brand, watermark, people, or misleading interface.

## Known deployment notes

- This build host has no Docker daemon, so the Dockerfile could not be executed here. The equivalent locked release build and zero-environment runtime were verified directly. The Dockerfile is multi-stage, does not use `.git`, runs as UID 10001, and copies only the release binary and `dist/` into Debian slim.
- Factory deployment supplies only `PORT`, so receipt copies remain in the durable queue until an SMTP relay is configured. The browser receipt and exports work fully without mail. This is the closest honest no-third-party implementation; configure the optional SMTP variables for automatic delivery.
- The factory must register the paid product and confirm its production price/return URL. No product ID or provider secret is hardcoded.

## Suggested next steps

1. Configure a trusted STARTTLS SMTP relay and perform a live inbox/deliverability check.
2. Run the factory container build and persistence-volume smoke in CI.
3. Register the Sociobot product, then test purchase, restore, refund, and revoked-license paths in staging.
