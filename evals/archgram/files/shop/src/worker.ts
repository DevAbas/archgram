import { Worker } from "bullmq";
import { saveOrder } from "./db.js";

new Worker("orders", async (job) => saveOrder(job.data), { connection: { url: process.env.REDIS_URL } });
