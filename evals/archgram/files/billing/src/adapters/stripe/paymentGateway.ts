import Stripe from "stripe";
import type { PaymentGateway } from "../../domain/ports.js";

const stripe = new Stripe(process.env.STRIPE_KEY ?? "");
export const paymentGateway: PaymentGateway = {
  async charge(customerId, amountCents) {
    await stripe.paymentIntents.create({ customer: customerId, amount: amountCents, currency: "eur", confirm: true });
  },
};
