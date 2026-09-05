<script lang="ts">
  import { onMount } from 'svelte';
  import Receipt from '../components/Receipt.svelte';
  import { api, type Decision, type Proposal } from '../lib';
  export let token: string;
  let proposal: Proposal | null = null, loading = true, error = '', working = false;
  let kind: Decision['kind'] | '' = '', respondentName = '', respondentEmail = '', note = '';
  onMount(load);
  async function load() {
    loading = true; error = '';
    try { proposal = await api<Proposal>(`/api/proposals/${token}`); respondentName = proposal.clientName; }
    catch (e) { error = e instanceof Error ? e.message : 'Could not open this proposal.'; }
    finally { loading = false; }
  }
  async function submit() {
    if (!kind) { error = 'Choose Accept, Request changes, or Decline.'; return; }
    working = true; error = '';
    try { proposal = await api<Proposal>(`/api/proposals/${token}/decision`, { method: 'POST', body: JSON.stringify({ kind, respondentName, respondentEmail, note }) }); window.scrollTo({ top: 0, behavior: 'smooth' }); }
    catch (e) { error = e instanceof Error ? e.message : 'Could not record the decision.'; }
    finally { working = false; }
  }
</script>

<section class="narrow client-page">
  {#if loading}<div class="state-page" aria-live="polite"><div class="pressed-loader" aria-hidden="true"></div><h1>Opening the proposal…</h1><p>Checking the private proposal link.</p></div>
  {:else if error && !proposal}<div class="state-page"><p class="eyebrow">Link unavailable</p><h1>This proposal could not be opened.</h1><p class="error" role="alert">{error}</p><button class="button" on:click={load}>Try again</button></div>
  {:else if proposal}
    <div class="client-title"><p class="eyebrow">Prepared for {proposal.clientName}</p><h1>{proposal.decision ? 'Decision recorded.' : 'Your decision is requested.'}</h1><p>{proposal.decision ? 'This frozen receipt is the shared record for both parties.' : `${proposal.freelancerName} has asked you to review this exact scope.`}</p></div>
    <Receipt {proposal} />
    {#if !proposal.decision}
      <form class="decision-form" on:submit|preventDefault={submit}>
        <fieldset><legend>Choose one final response</legend><div class="decision-options"><label class:chosen={kind === 'accepted'}><input type="radio" bind:group={kind} value="accepted" /><span><strong>Accept</strong><small>I acknowledge and approve this scope.</small></span></label><label class:chosen={kind === 'changes_requested'}><input type="radio" bind:group={kind} value="changes_requested" /><span><strong>Request changes</strong><small>I need an adjustment before approval.</small></span></label><label class:chosen={kind === 'declined'}><input type="radio" bind:group={kind} value="declined" /><span><strong>Decline</strong><small>I do not want to proceed with this scope.</small></span></label></div></fieldset>
        <div class="form-grid"><label>Your name<input bind:value={respondentName} required minlength="2" maxlength="80" autocomplete="name" /></label><label>Your email<input type="email" bind:value={respondentEmail} required autocomplete="email" /></label><label class="full">Decision note {kind === 'changes_requested' ? '(required)' : '(optional)'}<textarea bind:value={note} required={kind === 'changes_requested'} maxlength="2000" rows="4"></textarea></label></div>
        <p class="immutability"><strong>This decision is final.</strong> Submitting freezes the scope and creates a dated SHA-256 receipt. Contact {proposal.freelancerName} to discuss a new proposal.</p>
        {#if error}<p class="error" role="alert">{error}</p>{/if}<button class="button" type="submit" disabled={working}>{working ? 'Recording…' : 'Record final decision'}</button>
      </form>
    {:else}<p class="print-action"><button class="button secondary" type="button" on:click={() => window.print()}>Print or save as PDF</button></p>{/if}
  {/if}
</section>
