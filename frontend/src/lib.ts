export type Item = { label: string; description?: string; quantity: number; unitAmountCents: number };
export type Proposal = {
  id: string; title: string; freelancerName: string; clientName: string; message: string;
  currency: string; items: Item[]; createdAt: string; decision?: Decision | null;
};
export type Decision = { kind: 'accepted' | 'changes_requested' | 'declined'; respondentName: string; note: string; decidedAt: string; receiptHash: string };

export function money(cents: number, currency: string): string {
  try { return new Intl.NumberFormat(undefined, { style: 'currency', currency }).format(cents / 100); }
  catch { return `${currency} ${(cents / 100).toFixed(2)}`; }
}

export function total(items: Item[]): number {
  return items.reduce((sum, item) => sum + item.quantity * item.unitAmountCents, 0);
}

export function decisionLabel(kind: Decision['kind']): string {
  return { accepted: 'Accepted', changes_requested: 'Changes requested', declined: 'Declined' }[kind];
}

export async function api<T>(path: string, init?: RequestInit): Promise<T> {
  let response: Response;
  try { response = await fetch(path, { ...init, headers: { 'Content-Type': 'application/json', ...init?.headers } }); }
  catch { throw new Error('You appear to be offline. Reconnect and try again.'); }
  const data = await response.json().catch(() => ({}));
  if (!response.ok) throw new Error(data.error || `Request failed (${response.status}). Please try again.`);
  return data as T;
}

export const RECEIPT_NOTICE = 'This is a decision acknowledgement and scope record, not a regulated electronic signature.';
