import os

import pandas as pd
import psycopg


def run():
    rows = pd.read_parquet("staging/clean.parquet")
    with psycopg.connect(os.environ["WAREHOUSE_URL"]) as conn, conn.cursor() as cur:
        cur.executemany(
            "insert into sales_daily (store_id, day, amount) values (%s, %s, %s)",
            rows[["store_id", "day", "amount"]].itertuples(index=False),
        )
