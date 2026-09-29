import glob

import pandas as pd
import requests

POS_API = "https://api.example-pos.com/v2/sales"


def run():
    files = [pd.read_csv(path) for path in glob.glob("data/stores/*.csv")]
    online = pd.DataFrame(requests.get(POS_API, params={"since": "yesterday"}, timeout=30).json())
    pd.concat([*files, online]).to_parquet("staging/raw.parquet")
