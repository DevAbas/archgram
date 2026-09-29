export type Usage = { customerId: string; units: number };
export type Invoice = { customerId: string; amountCents: number };

export function computeInvoice(usage: Usage, centsPerUnit: number): Invoice {
  return { customerId: usage.customerId, amountCents: usage.units * centsPerUnit };
}
