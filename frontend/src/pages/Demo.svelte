<script lang="ts">
  import { onMount } from 'svelte';
  import Receipt from '../components/Receipt.svelte';
  import { api, type Decision, type Proposal } from '../lib';

  type Workspace = { id: string; expiresAt: string; proposal: Proposal };
  let workspace = '', proposal: Proposal = sampleProposal(), loading = true, error = '', working = false;
  let kind: Decision['kind'] | '' = '', respondentName = 'Maya Patel', respondentEmail = 'maya@mossandmorning.example', note = '';

  onMount(provision);

  function sampleProposal(): Proposal {
    return {
      id: 'CDR-DEMO-FERN', title: 'Spring website refresh', freelancerName: 'Fern Studio',
      clientName: 'Moss & Morning', message: 'This sample shows the scope a client reviews before work starts.',
      currency: 'USD', createdAt: new Date().toISOString(),
      items: [
        { label: 'Planning workshop', description: 'A 90-minute project workshop', quantity: 1, unitAmountCents: 65000 },
        { label: 'Homepage design', description: 'Desktop and mobile design direction', quantity: 1, unitAmountCents: 180000 },
        { label: 'Build handoff', description: 'Annotated files and implementation notes', quantity: 1, unitAmountCents: 75000 },
      ],
    };
  }

  async function provision() {
    loading = true; error = ''; kind = ''; note = ''; proposal = sampleProposal();
    try {
      const next = await api<Workspace>('/api/demo', { method: 'POST' });
      workspace = next.id; proposal = next.proposal; respondentName = proposal.clientName === 'Moss & Morning' ? 'Maya Patel' : proposal.clientName;
    } catch (e) {
      error = e instanceof Error ? e.message : 'The sample could not start. Try again.';
    } finally { loading = false; }
  }

  async function reset() {
    if (workspace) {
      try { await api(`/api/demo/${workspace}`, { method: 'DELETE' }); } catch { /* a reset may also remove an expired sample */ }
    }
    workspace = '';
    await provision();
  }

  async function submit() {
    if (!kind) { error = 'Choose Accept, Request changes, or Decline.'; return; }
    if (!workspace) { error = 'The sample is not ready yet. Reset the demo and try again.'; return; }
    working = true; error = '';
    try {
      const next = await api<Workspace>(`/api/demo/${workspace}/decision`, { method: 'POST', body: JSON.stringify({ kind, respondentName, respondentEmail, note }) });
      proposal = next.proposal;
      window.scrollTo({ top: 0, behavior: 'smooth' });
    } catch (e) { error = e instanceof Error ? e.message : 'Could not record the sample decision.'; }
    finally { working = false; }
  }
</script>

<section class="demo-banner" aria-label="Demo controls">
  <div><strong>Demo — sample data, nothing is saved</strong><span> This separate sample expires within 24 hours.</span></div>
  <div class="action-row"><button class="text-button" type="button" on:click={reset} disabled={loading}>Reset demo</button><a class="text-link" href="/#make">Start for real</a></div>
</section>

<section class="narrow client-page demo-page">
  <div class="client-title"><p class="eyebrow">Sample client decision</p><h1>{proposal.decision ? 'Sample decision recorded.' : 'Review this sample decision request.'}</h1><p>{proposal.decision ? 'The sample receipt now shows the final response.' : 'Try the same clear decision flow your client receives.'}</p></div>
  {#if loading}<p class="status" aria-live="polite">Loading an isolated sample workspace…</p>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <Receipt {proposal} />
  {#if !proposal.decision}
    <form class="decision-form" on:submit|preventDefault={submit}>
      <fieldset><legend>Choose one final response</legend><div class="decision-options"><label class:chosen={kind === 'accepted'}><input type="radio" bind:group={kind} value="accepted" /><span><strong>Accept</strong><small>I acknowledge and approve this scope.</small></span></label><label class:chosen={kind === 'changes_requested'}><input type="radio" bind:group={kind} value="changes_requested" /><span><strong>Request changes</strong><small>I need an adjustment before approval.</small></span></label><label class:chosen={kind === 'declined'}><input type="radio" bind:group={kind} value="declined" /><span><strong>Decline</strong><small>I do not want to proceed with this scope.</small></span></label></div></fieldset>
      <div class="form-grid"><label>Your name<input bind:value={respondentName} required minlength="2" maxlength="80" autocomplete="name" /></label><label>Your email<input type="email" bind:value={respondentEmail} required autocomplete="email" /></label><label class="full">Decision note {kind === 'changes_requested' ? '(required)' : '(optional)'}<textarea bind:value={note} required={kind === 'changes_requested'} maxlength="2000" rows="4"></textarea></label></div>
      <p class="immutability"><strong>This sample decision is final.</strong> It freezes this sample scope only. Reset the demo to start over.</p>
      <button class="button" type="submit" disabled={working || loading}>{working ? 'Recording…' : 'Record sample decision'}</button>
    </form>
  {:else}<p class="print-action"><button class="button secondary" type="button" on:click={() => window.print()}>Print or save as PDF</button></p>{/if}
</section>
