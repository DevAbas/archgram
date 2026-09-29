import { computeInvoice, type Usage } from "../domain/invoice.js";
import type { InvoiceRepository, Notifier, PaymentGateway } from "../domain/ports.js";

export type Ports = { invoices: InvoiceRepository; payments: PaymentGateway; notifier: Notifier };

export async function billInvoice(usage: Usage, ports: Ports): Promise<string> {
  const invoice = computeInvoice(usage, 12);
  const id = await ports.invoices.save(invoice);
  await ports.payments.charge(invoice.customerId, invoice.amountCents);
  await ports.notifier.receipt(invoice.customerId, id);
  return id;
}
