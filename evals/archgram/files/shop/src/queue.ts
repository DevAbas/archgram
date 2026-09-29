import { Queue } from "bullmq";

export const orders = new Queue("orders", { connection: { url: process.env.REDIS_URL } });
