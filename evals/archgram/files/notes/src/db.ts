import pg from "pg";

const pool = new pg.Pool({ connectionString: process.env.DATABASE_URL });
export const note = async (id: string) =>
  (await pool.query("select * from notes where id = $1", [id])).rows[0];
