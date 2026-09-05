<script lang="ts">
  import { onMount } from 'svelte';
  import Header from './components/Header.svelte';
  import Footer from './components/Footer.svelte';
  import Home from './pages/Home.svelte';
  import Client from './pages/Client.svelte';
  import Manage from './pages/Manage.svelte';
  import Demo from './pages/Demo.svelte';
  import Legal from './pages/Legal.svelte';
  const path = window.location.pathname.replace(/\/+$/, '') || '/';
  const clientToken = path.startsWith('/p/') ? path.slice(3) : '';
  const manageToken = path.startsWith('/m/') ? path.slice(3) : '';
  const pageTitle = path === '/' ? 'Client Decision Receipt — record client decisions'
    : path === '/demo' ? 'Demo — Client Decision Receipt'
    : path === '/privacy' ? 'Privacy — Client Decision Receipt'
    : path === '/terms' ? 'Terms — Client Decision Receipt'
    : clientToken ? 'Client decision — Client Decision Receipt'
    : manageToken ? 'Manage decision — Client Decision Receipt'
    : 'Page not found — Client Decision Receipt';
  let applyUpdate: (() => void) | null = null;
  onMount(() => {
    const handle = (event: Event) => { applyUpdate = (event as CustomEvent<() => void>).detail; };
    window.addEventListener('cdr-update-available', handle);
    return () => window.removeEventListener('cdr-update-available', handle);
  });
</script>

<svelte:head><title>{pageTitle}</title><link rel="canonical" href={`https://client-decision-receipt.sociobot.in${path}`} /></svelte:head>
<Header compact={path !== '/'} />
<main id="main" tabindex="-1">
  {#if path === '/'}<Home />
  {:else if path === '/demo'}<Demo />
  {:else if clientToken}<Client token={clientToken} />
  {:else if manageToken}<Manage token={manageToken} />
  {:else if path === '/privacy'}<Legal page="privacy" />
  {:else if path === '/terms'}<Legal page="terms" />
  {:else}<section class="narrow state-page"><p class="eyebrow">Page not found</p><h1>This page could not be found.</h1><p>Check the address or start a new decision request.</p><a class="button" href="/">Return home</a></section>{/if}
</main>
<Footer />
{#if applyUpdate}<aside class="update-notice" aria-live="polite"><span>An update is ready.</span><button class="button secondary" type="button" on:click={() => applyUpdate?.()}>Update page</button></aside>{/if}
