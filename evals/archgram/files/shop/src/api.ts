import express from "express";
import { products, saveOrder } from "./db.js";
import { cached } from "./cache.js";
import { orders } from "./queue.js";

const app = express();
app.get("/products", async (_req, res) => res.json(await cached("products", products)));
app.post("/orders", express.json(), async (req, res) => {
  await orders.add("order", req.body);
  res.status(202).end();
});
app.listen(3000);
