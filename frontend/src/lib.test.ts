import { describe, expect, it } from 'vitest';
import { decisionLabel, total } from './lib';

describe('receipt helpers', () => {
  it('totals quantities and unit prices', () => expect(total([
    { label: 'A', quantity: 2, unitAmountCents: 1250 },
    { label: 'B', quantity: 1, unitAmountCents: 500 },
  ])).toBe(3000));
  it('uses explicit decision labels', () => expect(decisionLabel('changes_requested')).toBe('Changes requested'));
});
