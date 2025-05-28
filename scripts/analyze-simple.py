import sys
import json

import pandas as pd

from process import *

try:
    file = sys.argv[1]
except IndexError:
    file = "results.csv"
try:
    out = sys.argv[2]
except IndexError:
    out = "lut.json"

df = process_simple(pd.read_csv(file))
df_grouped = df.groupby("opcode", group_keys=True)

def agg(data: pd.Series):
    bins = auto_bin_count(data)
    mode = binned_mode(data, bins=bins)
    return mode
agged = df_grouped["instruction_delta_per_op"].agg(agg)
print(agged)

with open(out, "w") as f:
    json.dump(agged.to_dict(), f, indent=2)
