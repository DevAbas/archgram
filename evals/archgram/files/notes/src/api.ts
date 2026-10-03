import express from "express";
import { note } from "./db.js";
import { sendDigest } from "./mail.js";

const app = express();
app.get("/notes/:id", async (req, res) => res.json(await note(req.params.id)));
app.post("/digest", async (_req, res) => {
  await sendDigest();
  res.status(202).end();
});
app.listen(3000);
