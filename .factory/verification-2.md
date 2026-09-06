# Independent verification 2 — FAIL

**Implementation candidate:** `5b0d32bc54c83b9bf3be26ce12f46e4788e2c4aa`  
**Documentation commit:** `524d163cf402aeeb4bedf78722e8d7d94dc3fab9`  
**Live URL:** https://client-decision-receipt.sociobot.in  
**Verified:** 2026-09-06 UTC

## Verdict

**FAIL — 2 findings; 2 untested public claims.** Do not mark this release as
complete until the two-party email job is operational and the public claims
are fully represented by sandbox tests.

The live health response reports `524d163cf402aeeb4bedf78722e8d7d94dc3fab9`,
not the implementation SHA. This is not a runtime-code discrepancy: the diff
from `5b0d32b` to `524d163` changes only `.factory/handoff.md`. The live
runtime was therefore assessed as the stated implementation candidate plus
its later report-only commit.

## Findings

### P1 — Final receipts are not emailed to either party

The brief's smallest useful product requires a dated receipt emailed to both
parties. A fresh live QA proposal was created, accepted, read through its
private management link, and deleted. Both delivery rows were
`sender_not_configured`; no message was sent to either supplied address. The
same state is described honestly in the UI and handoff, but it is still not an
end-to-end completion of the real job under the factory runtime, which supplies
only `PORT`.

**Required disposition:** configure a trusted SMTP sender in the product
runtime and verify delivery to both recipients, or formally change the scoped
product contract before re-verification. No credentials were inspected or
recorded.

### P1 — Claims manifest omits two public promises from runnable demo tests

All five declared claim commands pass, but the public copy makes two further
reliance claims not represented by `.factory/claims.json`:

1. The pricing section says **“Core decisions, receipt seals, JSON/CSV export,
   and deletion stay free.”** The `free-core` claim test exports JSON and
   deletes a record, but never exports CSV or asserts its free availability.
2. The README promises **“Durable two-recipient receipt delivery; automatic
   delivery when an SMTP sender is configured.”** There is no declared,
   fixture-backed claim test for this promise; live delivery is not testable in
   the supplied runtime because SMTP is absent.

Manual live evidence confirms CSV returned `200` with a CSV attachment, but
manual success does not satisfy the required claimed-outcome test contract.

## Passed verification evidence

### Clean checkout and declared commands

`npm ci` succeeded with no reported vulnerabilities. The following all passed
from the clean checkout:

- `npm test` — 2 Vitest tests, 7 Rust tests, and 8 Playwright tests.
- Each exact command listed in `.factory/claims.json` — all 5 passed:
  `demo-sandbox`, `frozen-receipt`, `no-tracking`, `offline-reload`, and
  `free-core`.
- `npm run test:claims` — 5/5 claim tests passed.
- `npm run build` — produced `dist/`; JS was 78.81 kB (28.60 kB gzip) and CSS
  was 16.92 kB (4.69 kB gzip).
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and
  `cargo build --release --locked`.

The Rust suite independently passed durable-snapshot restoration,
cross-instance shared-data-directory reads, demo separation, rate limiting,
and the concurrent-decision `201`/`409` regression.

### Live product exercise

- Fresh desktop and 390 px phone contexts showed the job (**Get a clear client
  decision**), audience (freelancers and small studios), and **Try it with
  sample data** before scrolling. The phone primary action began at y=409 px
  with no horizontal overflow.
- One click opened the realistic Fern Studio sample. Its persistent
  **Demo — sample data, nothing is saved** banner remained after an acceptance;
  its 64-character receipt seal appeared; **Reset demo** returned the fresh
  sample. The offline reload also worked after first visit.
- Ten repeated client reads and ten repeated management reads for a fresh live
  private record were all `200`; it was then deleted (`204`). This clears the
  earlier alternating-`404` persistence finding for the active one-replica
  deployment.
- Invalid full payload with a two-character title returned `400` and useful
  validation text. A boundary proposal with quantity `999` and unit amount
  `100000000` cents returned `201`. Two simultaneous final decisions returned
  `201` and `409`; the winning receipt had a 64-character SHA-256 seal. CSV
  export returned `200` with `Content-Disposition: attachment`; invalid delete
  confirmation returned `400`, correct deletion returned `204`, and the
  client link then returned `404`.
- A live burst of 60 unknown-token reads returned 45 `404`s and 15 `429`s;
  each sampled `429` supplied `Retry-After: 1`.
- `/health` returned `200` and a build SHA. `/demo`, `/privacy`, and `/terms`
  had their required route-specific titles. The designed `/404` response
  returned deliberate HTTP `404` with an explanation and route home. This is
  expected behaviour, not a defect.

### Accessibility, privacy, and site checks

- `verify-url.sh` passed against the live home page: HTTPS 200, title,
  language, one h1, main landmark, image alt coverage, and no console errors
  on the normal route.
- Playwright Axe on live home and demo found zero serious or critical issues.
  The Axe CLI itself could not launch its Selenium Chrome binary in this
  worker, so the installed Playwright Axe integration was used instead.
- The live skip link moved focus to `#main`. Reduced-motion media reduced
  transitions to `0.00001s`. Normal live home, demo, privacy, and terms routes
  had no console errors; the console entry on the deliberate 404 was the
  expected failed-resource notice for that 404 response.
- Browser request capture during the demo observed only the product origin.
  Legal links, mailto links, internal navigation, CSP, Referrer-Policy, and
  local runtime assets behaved as expected. No analytics or third-party runtime
  requests were observed.

## Earlier findings disposition

| Earlier item | Current disposition |
| --- | --- |
| Missing claim manifest and one-click demo | Fixed: manifest exists, five sandbox tests pass, and `/demo` is one click from the first screen. |
| Private links alternated 200/404 across replicas | Fixed for the active one-replica durable-snapshot deployment: 20/20 repeated private reads returned 200; local regression tests pass. |
| Simultaneous decisions returned 500 | Fixed: live and local results were one 201 and one 409. |
| `Retry-After: 0` | Fixed: live 429 responses supplied `Retry-After: 1`. |
| Skip-link focus | Fixed: live focus moved to `#main`. |
| Service-worker update strategy | Fixed in the shipped UI by its update action; offline reload claim passes. |
| SMTP receipt delivery absent | Still open; it is Finding 1 above. |

## Next steps

1. Configure SMTP and perform an inbox/deliverability check for both receipt
   recipients.
2. Add claim-manifest entries and demo-sandbox outcome tests for free CSV
   export and configured two-recipient delivery (a recorded SMTP fixture is
   acceptable for the latter), then re-run independent verification.
