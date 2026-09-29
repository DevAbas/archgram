import { Redis } from "ioredis";

const redis = new Redis(process.env.REDIS_URL ?? "redis://localhost:6379");
export async function cached<T>(key: string, load: () => Promise<T>): Promise<T> {
  const hit = await redis.get(key);
  if (hit) return JSON.parse(hit) as T;
  const value = await load();
  await redis.set(key, JSON.stringify(value), "EX", 60);
  return value;
}
