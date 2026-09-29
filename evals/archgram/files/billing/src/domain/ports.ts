import type { Invoice } from "./invoice.js";

export interface InvoiceRepository {
  save(invoice: Invoice): Promise<string>;
}
export interface PaymentGateway {
  charge(customerId: string, amountCents: number): Promise<void>;
}
export interface Notifier {
  receipt(customerId: string, invoiceId: string): Promise<void>;
}
