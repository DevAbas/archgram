import pg from "pg";
import type { InvoiceRepository } from "../../domain/ports.js";

const pool = new pg.Pool({ connectionString: process.env.DATABASE_URL });
export const invoiceRepository: InvoiceRepository = {
  async save(invoice) {
    const result = await pool.query("insert into invoices (customer_id, amount_cents) values ($1, $2) returning id", [invoice.customerId, invoice.amountCents]);
    return String(result.rows[0].id);
  },
};
