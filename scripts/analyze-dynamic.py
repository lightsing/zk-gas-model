import sys

import seaborn as sns
import matplotlib.pyplot as plt

from process import *

try:
    opcode = sys.argv[1]
except:
    print("usage: <Opcode> [input]")

try:
    file = sys.argv[2]
except IndexError:
    file = "results.csv"

df = process_simple(pd.read_csv(file))
df = clip_p5_p95(df)
df = df[(df["opcode"] == opcode) & (df["input_size"] <= 4096)]
df_grouped = df.groupby("input_size")["instruction_delta_per_op"]
agged = df_grouped.agg(agg_mode).reset_index()

x = agged["input_size"].values
y = agged["instruction_delta_per_op"].values

slope, intercept = np.polyfit(x, y, deg=1)
print(f"y = {slope}x + {intercept}")