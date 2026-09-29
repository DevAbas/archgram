import { ServerClient } from "postmark";
import type { Notifier } from "../../domain/ports.js";

const postmark = new ServerClient(process.env.POSTMARK_TOKEN ?? "");
export const notifier: Notifier = {
  async receipt(customerId, invoiceId) {
    await postmark.sendEmailWithTemplate({ From: "billing@example.com", To: customerId, TemplateAlias: "receipt", TemplateModel: { invoiceId } });
  },
};
