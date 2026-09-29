import express from "express";
import { billInvoice } from "../../app/billInvoice.js";
import { ports } from "../../wiring.js";

export const routes = express.Router();
routes.post("/invoices", express.json(), async (req, res) => {
  res.status(201).json({ id: await billInvoice(req.body, ports) });
});
