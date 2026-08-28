<script lang="ts">
  import { decisionLabel, money, total, RECEIPT_NOTICE, type Proposal } from '../lib';
  export let proposal: Proposal;
</script>

<article class="receipt" aria-labelledby="receipt-heading">
  <div class="receipt-top">
    <div><p class="eyebrow">Decision receipt</p><h2 id="receipt-heading">{proposal.title}</h2></div>
    {#if proposal.decision}<span class:accepted={proposal.decision.kind === 'accepted'} class:changes={proposal.decision.kind === 'changes_requested'} class:declined={proposal.decision.kind === 'declined'} class="decision-stamp">{decisionLabel(proposal.decision.kind)}</span>{/if}
  </div>
  <dl class="metadata">
    <div><dt>Catalogue no.</dt><dd>{proposal.id}</dd></div>
    <div><dt>Prepared by</dt><dd>{proposal.freelancerName}</dd></div>
    <div><dt>Prepared for</dt><dd>{proposal.clientName}</dd></div>
    <div><dt>Issued</dt><dd>{new Date(proposal.createdAt).toLocaleString()}</dd></div>
    {#if proposal.decision}<div><dt>Recorded</dt><dd>{new Date(proposal.decision.decidedAt).toLocaleString()}</dd></div><div><dt>Respondent</dt><dd>{proposal.decision.respondentName}</dd></div>{/if}
  </dl>
  {#if proposal.message}<p class="message">{proposal.message}</p>{/if}
  <div class="table-wrap">
    <table>
      <caption>Frozen scope and fees</caption>
      <thead><tr><th>Scope item</th><th>Qty</th><th>Unit</th><th>Amount</th></tr></thead>
      <tbody>{#each proposal.items as item}<tr><td><strong>{item.label}</strong>{#if item.description}<small>{item.description}</small>{/if}</td><td>{item.quantity}</td><td>{money(item.unitAmountCents, proposal.currency)}</td><td>{money(item.unitAmountCents * item.quantity, proposal.currency)}</td></tr>{/each}</tbody>
      <tfoot><tr><th colspan="3">Scope total</th><td>{money(total(proposal.items), proposal.currency)}</td></tr></tfoot>
    </table>
  </div>
  {#if proposal.decision}
    {#if proposal.decision.note}<section class="decision-note"><h3>Decision note</h3><p>{proposal.decision.note}</p></section>{/if}
    <div class="seal"><span>SHA-256 receipt seal</span><code>{proposal.decision.receiptHash}</code></div>
  {/if}
  <p class="legal-note">{RECEIPT_NOTICE}</p>
</article>
