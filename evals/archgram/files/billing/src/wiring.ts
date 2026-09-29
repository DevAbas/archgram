import { invoiceRepository } from "./adapters/postgres/invoiceRepository.js";
import { notifier } from "./adapters/email/notifier.js";
import { paymentGateway } from "./adapters/stripe/paymentGateway.js";
import type { Ports } from "./app/billInvoice.js";

export const ports: Ports = { invoices: invoiceRepository, payments: paymentGateway, notifier };
