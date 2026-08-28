<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib';
  import { checkoutUrl, initializeLicense, restoreLicense, type LicenseState } from '../license';

  type DraftItem = { label: string; description: string; quantity: number; price: number };
  type Created = { id: string; clientPath: string; managePath: string };
  type ArchiveItem = { id: string; title: string; managePath: string; createdAt: string };
  let title = '', freelancerName = '', freelancerEmail = '', clientName = '', clientEmail = '', message = '', currency = 'USD';
  let items: DraftItem[] = [{ label: '', description: '', quantity: 1, price: 0 }];
  let working = false, error = '', created: Created | null = null, copied = '';
  let archive: ArchiveItem[] = [];
  let license: LicenseState = { unlocked: false, checking: true, notice: '' };
  let licenseInput = '';

  onMount(async () => {
    archive = readArchive();
    license = await initializeLicense();
  });

  function readArchive(): ArchiveItem[] {
    try { return JSON.parse(localStorage.getItem('cdr:archive') || '[]'); } catch { return []; }
  }
  function addItem() { items = [...items, { label: '', description: '', quantity: 1, price: 0 }]; }
  function removeItem(index: number) { if (items.length > 1) items = items.filter((_, i) => i !== index); }
  async function create() {
    error = ''; working = true;
    try {
      created = await api<Created>('/api/proposals', { method: 'POST', body: JSON.stringify({
        title, freelancerName, freelancerEmail, clientName, clientEmail, message, currency,
        items: items.map(item => ({ label: item.label, description: item.description, quantity: Number(item.quantity), unitAmountCents: Math.round(Number(item.price) * 100) })),
      }) });
      archive = [{ id: created.id, title, managePath: created.managePath, createdAt: new Date().toISOString() }, ...archive.filter(a => a.id !== created?.id)];
      localStorage.setItem('cdr:archive', JSON.stringify(archive));
      setTimeout(() => document.querySelector<HTMLElement>('#created-heading')?.focus(), 0);
    } catch (e) { error = e instanceof Error ? e.message : 'Could not create the proposal.'; }
    finally { working = false; }
  }
  async function copy(path: string, label: string) {
    try { await navigator.clipboard.writeText(`${location.origin}${path}`); copied = label; setTimeout(() => copied = '', 2400); }
    catch { copied = 'Copy unavailable—select the link instead'; }
  }
  async function restore() {
    if (!licenseInput.trim()) return;
    license = { ...license, checking: true };
    license = await restoreLicense(licenseInput);
  }
  function exportArchive() {
    const blob = new Blob([JSON.stringify({ exportedAt: new Date().toISOString(), records: archive }, null, 2)], { type: 'application/json' });
    const link = document.createElement('a');
    link.href = URL.createObjectURL(blob); link.download = 'decision-receipt-archive.json'; link.click(); URL.revokeObjectURL(link.href);
  }
  $: proposalUrl = created ? `${location.origin}${created.clientPath}` : '';
  $: emailHref = `mailto:${encodeURIComponent(clientEmail)}?subject=${encodeURIComponent(`Decision requested: ${title}`)}&body=${encodeURIComponent(`Hi ${clientName},\n\nPlease review the frozen scope and record Accept, Request changes, or Decline here:\n${proposalUrl}\n\nThis is an acknowledgement, not an electronic signature.\n\nThanks,\n${freelancerName}`)}`;
</script>

<section class="hero">
  <div class="hero-copy">
    <p class="eyebrow">A field record for client work</p>
    <h1>Turn “looks good” into a decision you can keep.</h1>
    <p class="lead">Send one quiet link. Your client accepts, requests changes, or declines. The exact scope is frozen into a dated, verifiable receipt—without open tracking.</p>
    <a class="button" href="#make">Make a decision link <span aria-hidden="true">↓</span></a>
    <p class="micro">Free for occasional proposals · no account required</p>
  </div>
  <figure class="hero-plate">
    <picture><source media="(max-width: 640px)" srcset="/herbarium-receipt-640.webp" /><img src="/herbarium-receipt-960.webp" width="960" height="640" alt="A pressed maidenhair fern, archival card, and blank catalogue tag on herbarium paper" decoding="async" fetchpriority="high" /></picture>
    <figcaption><span>Plate 01</span> Scope, pressed into the record.</figcaption>
  </figure>
</section>

<section class="process" aria-labelledby="process-heading">
  <div><p class="eyebrow">The useful signal</p><h2 id="process-heading">A decision, not a view count.</h2></div>
  <ol><li><span>01</span><strong>List the agreed scope</strong><p>Add deliverables and fees. We freeze exactly what the client sees.</p></li><li><span>02</span><strong>Send the private link</strong><p>No login, surveillance pixel, or signature theatre.</p></li><li><span>03</span><strong>Keep the receipt</strong><p>Both sides see the decision, timestamp, and SHA-256 seal.</p></li></ol>
</section>

<section id="make" class="maker" aria-labelledby="maker-heading">
  <div class="section-intro"><p class="eyebrow">New specimen</p><h2 id="maker-heading">Prepare a decision request</h2><p>The decision link is for your client. The separate management link is the only key to exports and deletion.</p></div>
  {#if created}
    <div class="success-sheet" aria-live="polite">
      <span class="specimen-number">{created.id}</span>
      <h3 id="created-heading" tabindex="-1">Your links are ready.</h3>
      <p>Save the management link now. For privacy, it can’t be recovered if you lose it.</p>
      <div class="link-field"><label for="client-link">Client decision link</label><div><input id="client-link" readonly value={proposalUrl} /><button type="button" class="button secondary" on:click={() => copy(created!.clientPath, 'Client link copied')}>Copy</button></div></div>
      <div class="link-field"><label for="manage-link">Your private management link</label><div><input id="manage-link" readonly value={`${location.origin}${created.managePath}`} /><button type="button" class="button secondary" on:click={() => copy(created!.managePath, 'Management link copied')}>Copy</button></div></div>
      <p class="status" aria-live="polite">{copied}</p>
      <div class="action-row"><a class="button" href={emailHref}>Email client</a><a class="text-link" href={created.managePath}>Open management page</a><button class="text-button" type="button" on:click={() => { created = null; title = ''; }}>Create another</button></div>
    </div>
  {:else}
    <form on:submit|preventDefault={create}>
      <fieldset><legend>People and proposal</legend><div class="form-grid"><label>Proposal title<input bind:value={title} required minlength="3" maxlength="120" autocomplete="off" /></label><label>Currency<select bind:value={currency}><option>USD</option><option>EUR</option><option>GBP</option><option>CAD</option><option>AUD</option><option>INR</option></select></label><label>Your name<input bind:value={freelancerName} required maxlength="80" autocomplete="name" /></label><label>Your email<input type="email" bind:value={freelancerEmail} required autocomplete="email" /></label><label>Client name<input bind:value={clientName} required maxlength="80" autocomplete="off" /></label><label>Client email<input type="email" bind:value={clientEmail} required autocomplete="off" /></label><label class="full">Short introduction <span>(optional)</span><textarea bind:value={message} maxlength="2000" rows="3"></textarea></label></div></fieldset>
      <fieldset><legend>Frozen scope</legend><p class="field-help">Amounts stay fixed on the receipt after a decision.</p>
        <div class="items">{#each items as item, index}<div class="item-row"><span class="item-no">{String(index + 1).padStart(2, '0')}</span><label>Scope item<input bind:value={item.label} required minlength="2" maxlength="160" /></label><label class="item-description">Details <span>(optional)</span><input bind:value={item.description} maxlength="500" /></label><label>Qty<input type="number" bind:value={item.quantity} min="1" max="999" required inputmode="numeric" /></label><label>Unit price<input type="number" bind:value={item.price} min="0" max="1000000" step="0.01" required inputmode="decimal" /></label><button type="button" class="remove" on:click={() => removeItem(index)} disabled={items.length === 1} aria-label={`Remove scope item ${index + 1}`}>Remove</button></div>{/each}</div>
        <button class="button secondary add" type="button" on:click={addItem} disabled={items.length >= 30}>+ Add scope item</button>
      </fieldset>
      {#if error}<p class="error" role="alert">{error}</p>{/if}
      <div class="submit-row"><p>By creating a link, you agree to the <a href="/terms">terms</a>. This is an acknowledgement—not an e-signature.</p><button class="button" type="submit" disabled={working}>{working ? 'Preparing…' : 'Create decision link'}</button></div>
    </form>
  {/if}
</section>

<section id="archive" class="archive" aria-labelledby="archive-heading">
  <div class="section-intro"><p class="eyebrow">This browser</p><h2 id="archive-heading">Your private archive</h2><p>Management links are stored only in this browser. Export a copy for safekeeping.</p></div>
  {#if archive.length}<ul>{#each archive as entry}<li><div><strong>{entry.title}</strong><small>{entry.id} · {new Date(entry.createdAt).toLocaleDateString()}</small></div><a class="button secondary" href={entry.managePath}>Open record</a></li>{/each}</ul>{#if license.unlocked}<p class="archive-export"><button class="button secondary" type="button" on:click={exportArchive}>Export archive index</button></p>{/if}{:else}<div class="empty"><span aria-hidden="true">⌁</span><p><strong>No pressed specimens yet.</strong><br />Create your first decision link above; its management key will appear here.</p></div>{/if}
</section>

<section id="pricing" class="pricing" aria-labelledby="pricing-heading">
  <div><p class="eyebrow">Simple field kit</p><h2 id="pricing-heading">Free to decide. Pro to make it yours.</h2><p>Core decisions, receipt seals, JSON/CSV export, and deletion stay free. Always.</p></div>
  <div class="price-sheet"><p class="price"><span>$29</span> one time</p><h3>Independent Pro</h3><ul><li>Export your browser archive index in one click</li><li>Priority product support</li><li>Future studio-branding controls when released</li></ul>{#if license.unlocked}<p class="license-good">✓ Pro is unlocked on this device.</p><button class="button secondary" type="button" on:click={exportArchive}>Export archive index</button><p><a href="mailto:priority@sociobot.in">Contact priority support</a></p>{:else}<a class="button" href={checkoutUrl}>Buy Pro once</a>{/if}<p class="fine">Sociobot/Dodo is merchant of record. Refunds are handled there and revoke the license.</p><details><summary>Have a license? Restore purchase</summary><label for="license">License token</label><div class="restore"><input id="license" bind:value={licenseInput} autocomplete="off" /><button class="button secondary" type="button" disabled={license.checking} on:click={restore}>Verify</button></div>{#if license.notice}<p class="notice" aria-live="polite">{license.notice} <a href={checkoutUrl}>Get a license</a></p>{/if}</details></div>
</section>
