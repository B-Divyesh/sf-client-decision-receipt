<script lang="ts">
  import Header from './components/Header.svelte';
  import Footer from './components/Footer.svelte';
  import Home from './pages/Home.svelte';
  import Client from './pages/Client.svelte';
  import Manage from './pages/Manage.svelte';
  import Legal from './pages/Legal.svelte';
  const path = window.location.pathname.replace(/\/+$/, '') || '/';
  const clientToken = path.startsWith('/p/') ? path.slice(3) : '';
  const manageToken = path.startsWith('/m/') ? path.slice(3) : '';
</script>

<Header compact={path !== '/'} />
<main id="main">
  {#if path === '/'}<Home />
  {:else if clientToken}<Client token={clientToken} />
  {:else if manageToken}<Manage token={manageToken} />
  {:else if path === '/privacy'}<Legal page="privacy" />
  {:else if path === '/terms'}<Legal page="terms" />
  {:else}<section class="narrow state-page"><p class="eyebrow">Field note 404</p><h1>This page isn’t in the archive.</h1><p>The link may be incomplete or the receipt may have been deleted.</p><a class="button" href="/">Return home</a></section>{/if}
</main>
<Footer />
