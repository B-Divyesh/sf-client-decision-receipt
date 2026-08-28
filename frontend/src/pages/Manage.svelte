<script lang="ts">
  import { onMount } from 'svelte';
  import Receipt from '../components/Receipt.svelte';
  import { api, type Proposal } from '../lib';
  export let token: string;
  type Managed = Proposal & { clientPath: string; delivery: { recipient: string; status: string; createdAt: string }[] };
  let record: Managed | null = null, loading = true, error = '', copied = '', confirmOpen = false, confirmation = '', deleting = false, deleted = false;
  onMount(load);
  async function load() { loading = true; error = ''; try { record = await api<Managed>(`/api/manage/${token}`); } catch (e) { error = e instanceof Error ? e.message : 'Could not open this record.'; } finally { loading = false; } }
  async function copy() { if (!record) return; try { await navigator.clipboard.writeText(`${location.origin}${record.clientPath}`); copied = 'Client link copied.'; } catch { copied = 'Copy unavailable—select the link instead.'; } }
  async function remove() {
    deleting = true; error = '';
    try {
      await api(`/api/manage/${token}`, { method: 'DELETE', body: JSON.stringify({ confirmation }) });
      const archive = JSON.parse(localStorage.getItem('cdr:archive') || '[]').filter((entry: { managePath: string }) => entry.managePath !== `/m/${token}`);
      localStorage.setItem('cdr:archive', JSON.stringify(archive)); deleted = true;
    } catch (e) { error = e instanceof Error ? e.message : 'Could not delete this record.'; }
    finally { deleting = false; }
  }
</script>

<section class="narrow manage-page">
  {#if loading}<div class="state-page" aria-live="polite"><div class="pressed-loader" aria-hidden="true"></div><h1>Opening your private record…</h1></div>
  {:else if deleted}<div class="state-page"><p class="eyebrow">Record removed</p><h1>The proposal data was deleted.</h1><p>The personal data and links are gone. If it had a receipt, only its anonymous hash seal remains as an integrity tombstone.</p><a class="button" href="/">Return to your archive</a></div>
  {:else if error && !record}<div class="state-page"><p class="eyebrow">Private link unavailable</p><h1>This record could not be opened.</h1><p class="error" role="alert">{error}</p><button class="button" on:click={load}>Try again</button></div>
  {:else if record}
    <div class="manage-title"><p class="eyebrow">Private management view</p><h1>{record.decision ? 'A decision is on record.' : 'Awaiting the client.'}</h1><p>{record.decision ? 'Export the frozen record for your files.' : 'Views are not tracked. The next useful signal is an explicit response.'}</p></div>
    <div class="manager-tools"><div class="link-field"><label for="managed-client-link">Client decision link</label><div><input id="managed-client-link" readonly value={`${location.origin}${record.clientPath}`} /><button class="button secondary" type="button" on:click={copy}>Copy</button></div><p class="status" aria-live="polite">{copied}</p></div><div class="action-row"><a class="button secondary" href={`/api/manage/${token}/export.json`}>Export JSON</a><a class="button secondary" href={`/api/manage/${token}/export.csv`}>Export CSV</a><button class="text-button danger-link" type="button" on:click={() => confirmOpen = true}>Delete data</button></div></div>
    <Receipt proposal={record} />
    {#if record.decision}<section class="delivery"><h2>Receipt delivery</h2><p>Two email-ready copies are retained in the durable delivery queue.</p>{#if record.delivery.length}<ul>{#each record.delivery as delivery}<li><span>{delivery.recipient}</span><strong>{delivery.status}</strong></li>{/each}</ul>{/if}</section>{/if}
    {#if confirmOpen}<div class="confirm-panel" role="alertdialog" aria-modal="true" aria-labelledby="delete-heading"><h2 id="delete-heading">Permanently delete “{record.title}”?</h2><p>Proposal details, contacts, delivery copies, and both links will be removed. Export first if needed. The anonymous receipt hash is retained for integrity.</p><label for="confirmation">Type <strong>DELETE</strong> to confirm</label><input id="confirmation" bind:value={confirmation} autocomplete="off" /><div class="action-row"><button class="button danger" on:click={remove} disabled={confirmation !== 'DELETE' || deleting}>{deleting ? 'Deleting…' : 'Delete permanently'}</button><button class="button secondary" on:click={() => { confirmOpen = false; confirmation = ''; }}>Keep record</button></div>{#if error}<p class="error" role="alert">{error}</p>{/if}</div>{/if}
  {/if}
</section>
