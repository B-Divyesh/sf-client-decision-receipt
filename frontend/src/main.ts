import { mount } from 'svelte';
import App from './App.svelte';
import './styles.css';

mount(App, { target: document.getElementById('app')! });

if ('serviceWorker' in navigator && import.meta.env.PROD) {
  window.addEventListener('load', async () => {
    const registration = await navigator.serviceWorker.register('/sw.js');
    const announce = () => {
      if (!registration.waiting || !navigator.serviceWorker.controller) return;
      window.dispatchEvent(new CustomEvent('cdr-update-available', {
        detail: () => registration.waiting?.postMessage({ type: 'apply-update' }),
      }));
    };
    registration.addEventListener('updatefound', () => registration.installing?.addEventListener('statechange', announce));
    announce();
    navigator.serviceWorker.addEventListener('controllerchange', () => window.location.reload());
  });
}
