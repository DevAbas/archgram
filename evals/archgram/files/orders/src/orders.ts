import express from "express";

const app = express();
const orders: unknown[] = [];
app.post("/orders", express.json(), (req, res) => {
  orders.push(req.body);
  res.status(201).json({ id: orders.length });
});
app.listen(3000);
