import pandas as pd


def run():
    rows = pd.read_parquet("staging/raw.parquet")
    valid = rows["amount"].notna() & (rows["amount"] >= 0) & rows["store_id"].notna()
    rows[~valid].to_parquet("quarantine/bad_rows.parquet")
    rows[valid].to_parquet("staging/clean.parquet")
