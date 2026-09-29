import express from "express";
import pg from "pg";
import { Client } from "@elastic/elasticsearch";

const db = new pg.Pool({ connectionString: process.env.DATABASE_URL });
const search = new Client({ node: process.env.ELASTIC_URL });
const app = express();
app.get("/articles/:id", async (req, res) =>
  res.json((await db.query("select * from articles where id = $1", [req.params.id])).rows[0]));
app.get("/search", async (req, res) =>
  res.json(await search.search({ index: "articles", q: String(req.query.q) })));
app.listen(3000);
