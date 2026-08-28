<script lang="ts">export let page: 'privacy' | 'terms';</script>

<article class="legal narrow">
  {#if page === 'privacy'}
    <p class="eyebrow">Last updated 28 August 2026</p><h1>Privacy, kept plain.</h1>
    <p class="lead">Client Decision Receipt stores only what is needed to create, decide, export, and delete a proposal record. It does not use tracking pixels or advertising analytics.</p>
    <h2>What we store</h2><p>Proposal titles, names, email addresses, notes, scope items, amounts, timestamps, the recorded decision, and its SHA-256 hash. Your browser locally stores private management links and, if purchased, your Sociobot license token and a daily verification result.</p>
    <h2>Why and for how long</h2><p>We use this data solely to provide the decision workflow and exports. Records remain until the holder of the private management link deletes them. Deletion removes proposal and contact data; an anonymous receipt hash, catalogue number, and dates may remain to preserve integrity.</p>
    <h2>Who receives data</h2><p>The freelancer and named client can access their respective link. Receipt delivery copies are stored in the service queue for the two supplied addresses. For Pro purchases, Sociobot/Dodo acts as merchant of record; payment details never reach this product.</p>
    <h2>Your choices</h2><p>Use the private management link to export JSON or CSV and permanently delete a record. Email <a href="mailto:privacy@sociobot.in">privacy@sociobot.in</a> if you lose access or need help.</p>
  {:else}
    <p class="eyebrow">Last updated 28 August 2026</p><h1>Terms of use.</h1>
    <p class="lead">This service records a plain-language business decision. It is not a regulated electronic-signature service, legal advice, invoicing, or proof that a person had authority to bind an organisation.</p>
    <h2>Your responsibility</h2><p>You must have permission to enter the contact information and scope you submit. Check amounts and terms before sharing. Anyone with a private client link can record its one final decision; anyone with the management link can view, export, or delete the record.</p>
    <h2>Receipt integrity</h2><p>After a decision, the scope and response become read-only and receive a SHA-256 seal. The seal can reveal later changes; it does not verify identity or replace legal execution requirements.</p>
    <h2>Availability and acceptable use</h2><p>Do not use the service for unlawful, deceptive, abusive, or high-risk regulated agreements. We may limit abusive traffic and cannot promise uninterrupted availability. Export important records.</p>
    <h2>Pro purchase</h2><p>Independent Pro is a $29 one-time license for the listed cosmetic and support features. Sociobot/Dodo is merchant of record and handles checkout and refunds. A refunded, expired, or revoked license becomes inactive without affecting free exports or safety features.</p>
    <h2>Contact</h2><p>Questions can be sent to <a href="mailto:support@sociobot.in">support@sociobot.in</a>.</p>
  {/if}
</article>
