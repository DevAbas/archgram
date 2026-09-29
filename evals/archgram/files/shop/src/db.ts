import pg from "pg";

const pool = new pg.Pool({ connectionString: process.env.DATABASE_URL });
export const products = async () => (await pool.query("select * from products")).rows;
export const saveOrder = (order: unknown) => pool.query("insert into orders (body) values ($1)", [order]);
