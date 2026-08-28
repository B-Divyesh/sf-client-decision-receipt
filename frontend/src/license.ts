const SLUG = 'client-decision-receipt';
const KEY = `sb_license:${SLUG}`;
const CACHE_KEY = `${KEY}:verdict`;
export const checkoutUrl = `https://api.sociobot.in/api/v1/products/${SLUG}/checkout`;

export type LicenseState = { unlocked: boolean; checking: boolean; notice: string };

export async function initializeLicense(): Promise<LicenseState> {
  const params = new URLSearchParams(location.search);
  const returned = params.get('license');
  if (returned) {
    localStorage.setItem(KEY, returned);
    params.delete('license');
    history.replaceState({}, '', `${location.pathname}${params.size ? `?${params}` : ''}${location.hash}`);
  }
  const token = returned || localStorage.getItem(KEY);
  if (!token) return { unlocked: false, checking: false, notice: '' };
  const cached = JSON.parse(localStorage.getItem(CACHE_KEY) || 'null') as { valid: boolean; checkedAt: number } | null;
  const fresh = cached && Date.now() - cached.checkedAt < 86_400_000;
  if (fresh) return { unlocked: cached.valid, checking: false, notice: cached.valid ? '' : 'License no longer active.' };
  try {
    const response = await fetch(`https://api.sociobot.in/api/v1/products/${SLUG}/verify?license=${encodeURIComponent(token)}`);
    const result = await response.json();
    const valid = response.ok && result.valid === true;
    localStorage.setItem(CACHE_KEY, JSON.stringify({ valid, checkedAt: Date.now() }));
    return { unlocked: valid, checking: false, notice: valid ? '' : 'License no longer active.' };
  } catch {
    return { unlocked: cached?.valid === true, checking: false, notice: 'License check will resume when you are online.' };
  }
}

export async function restoreLicense(token: string): Promise<LicenseState> {
  localStorage.setItem(KEY, token.trim());
  localStorage.removeItem(CACHE_KEY);
  return initializeLicense();
}
