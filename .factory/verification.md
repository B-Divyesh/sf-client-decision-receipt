# Independent verification — FAIL

**Candidate:** `75019d3cb52948843ff2f2db890cf01a2c13342b`  
**Live URL:** https://client-decision-receipt.sociobot.in  
**Verified:** 2026-08-28 UTC from a clean checkout

## Release decision

**FAIL — do not release/promote this candidate.** The deployed service is not
reliably usable: successive requests for the same newly created private link
alternate between a valid record and `404`. This is consistent with multiple
instances using separate SQLite databases and separately generated HMAC
secrets. It also fails the mandatory claims and first-read/demo gates.

## Mandatory gates

1. **FAIL / P1 — claims contract absent.** The very first check was for
   `.factory/claims.json`; it is missing. Consequently there were no listed
   claim tests to execute from the demo entry point. The work order expressly
   defines a missing manifest as release-blocking.
2. **FAIL / P1 — first-read and demo-sandbox gate.** A cold desktop visit
   rendered “Turn ‘looks good’ into a decision you can keep.” It explains that
   a client can accept, request changes, or decline a scope, and the first
   apparent action is **Make a decision link**. “For whom” is only implied by
   “client work”/“your client”, rather than stated plainly as freelancers or
   studios. More importantly, there is no one-click **Try it with sample
   data** action anywhere on the home page (buttons are Remove, Add scope
   item, and Create decision link). This alone is a specified FAIL condition.

## Blocking defects

### P0 — live private records are split across backend instances

On the live URL I created an ephemeral QA proposal, then sent eight sequential
GETs for each returned capability URL. Results were:

| Same capability URL | Observed HTTP statuses |
| --- | --- |
| Client decision URL | `404,200,404,200,404,200,404,200` |
| Management URL | `404,200,404,200,404,200,404,200` |

The candidate health endpoint reports the exact candidate SHA
`75019d3cb52948843ff2f2db890cf01a2c13342b`, so this is not an older frontend
or deployment. The source uses local `/data/receipts.sqlite` and a generated
`/data/instance-secret`; the live alternation proves those are not shared by
the instances behind the ingress. This makes real client and management links
work only intermittently, isolates queued receipts, and makes deletion
instance-dependent. The QA record was subsequently retried for deletion until
the owning instance returned `204`; following reads were all `404`.

### P1 — required emailed receipt delivery is not operational with factory runtime

The researched brief requires the frozen receipt to be emailed to both
parties. With the required no-extra-environment runtime, source and local
verification show only two durable outbox rows with status `queued`; SMTP is
optional and the deployed factory contract supplies only `PORT`. Thus there
is no automatic email delivery in the release configuration. This is a
material missing part of the smallest useful product, independent of the
replica failure.

### P1 — simultaneous final decisions return an internal error

Against a fresh local release build and temporary database, two concurrent
decision submissions for one client token returned `201,500`. The successful
request created a 64-character SHA-256 receipt; the other should have returned
the documented immutable-decision conflict (`409`) but instead hit SQLite
locking and exposed a recoverable server failure. A losing client is told to
retry rather than receiving the final receipt state.

## Verification evidence

### Clean install, tests, and build

- `npm ci`: passed; 0 npm audit vulnerabilities reported.
- `npm test`: passed — 2 Vitest tests, 3 Rust tests, and 2 Playwright journeys.
- `cargo fmt -- --check`: passed.
- `cargo clippy --all-targets -- -D warnings`: passed.
- `npm run build`: passed; `dist/` produced.
- `cargo build --release --locked`: passed.
- Built assets: JS 71,869 B (26,980 B gzip), CSS 15,712 B (4,440 B gzip),
  mobile hero 22,138 B. These meet the stated static budgets.

### Local end-to-end/API exercise

- Invalid short title: `400 Proposal title must be 3–120 characters.`
- Maximum accepted boundary (quantity `999`, unit amount `100000000` cents):
  `201`; client view kept that exact scope and intentionally hid party emails.
- Changes request with no note: `400 Describe the changes you need.`
- Normal final decision: receipt seal length 64; management view showed two
  queued delivery records; JSON export returned an attachment.
- Incorrect deletion confirmation: `400`; correct `DELETE`: `204`; subsequent
  client read: `404`.
- Read-limit burst of 60 local requests: 20 `429`s; first observed at request
  40 with `Retry-After: 0`.

### Live deployment exercise and identity

- `/health` returned
  `{"buildSha":"75019d3cb52948843ff2f2db890cf01a2c13342b","status":"ok"}`.
- Live `index-MQ-ac2ya.js`, `index-B14jffaO.css`, and `sw.js` match the
  candidate build/source; the latter’s SHA-256 was identical locally and live.
- An ephemeral live changes-request flow created a receipt (`201`, 64-char
  seal), CSV export (`200` attachment), and deletion (`204`), but the client
  and management reads in that same flow hit the alternating-instance `404`s.
- A live burst of 60 valid-looking unknown proposal reads produced 18 `429`s;
  the first observed at request 29 and included `Retry-After: 0`. Rate limiting
  therefore exists, though the zero-second retry value offers clients no useful
  backoff delay.

### Privacy, security, PWA, accessibility, and performance

- Cold-page requests and browser network capture were same-origin only; no
  analytics, fonts, scripts, or tracker calls were observed. The only allowed
  future external connection is the Sociobot billing API.
- Live responses set CSP (self-only with the documented Sociobot connect
  allowance), `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`,
  permissions policy, and immutable one-year caching for hashed assets.
- Desktop and 390×844 mobile had no console/page errors or horizontal overflow.
  The dark and light checks had a designed 3 px visible focus ring; reduced
  motion reduced transitions/animations to 0.01 ms.
- Axe found **zero serious or critical violations** on the live home page in
  desktop, mobile, and dark-mode checks. It has one `h1`, `main`, title, and
  `lang=en`.
- Service worker registered and a cached offline reload returned the home page
  successfully. Static review found no `skipWaiting`, so an already open tab
  will wait to receive a future worker update (P2 follow-up).
- Mobile Lighthouse on live: Performance **94**, Accessibility **100**, Best
  Practices **100**, SEO **100**; LCP 1,201 ms, CLS 0, TBT 300 ms.

## Other follow-ups (non-blocking relative to the above)

- **P2:** the skip link scrolls to `#main` but `<main>` is not programmatically
  focusable, so focus remains on the skip link after activation.
- **P2:** document and test an actual shared durable database/secret volume (or
  use a shared database) across replicas, including cross-instance create/read,
  decision, export, and deletion tests before re-verification.
- **P2:** add a service-worker update test and an explicit update strategy.

## Required remediation before another verification

1. Add `.factory/claims.json` with runnable claim tests through a one-click
   sample-data demo; add the demo to the cold landing screen and plainly name
   freelancers/studios as the audience.
2. Make proposal data and the instance HMAC secret shared/persistent across all
   deployed instances, then prove a link works across load-balanced requests.
3. Configure and test actual two-party email delivery under the factory runtime,
   or honestly redesign the product/brief contract before release.
4. Handle SQLite contention so every losing decision deterministically returns
   `409`, never `500`; add a concurrent integration test.
