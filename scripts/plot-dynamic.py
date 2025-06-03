import sys

import seaborn as sns
import matplotlib.pyplot as plt

from process import *

try:
    file = sys.argv[1]
except IndexError:
    file = "results.csv"

df = process_simple(pd.read_csv(file))
df = clip_p5_p95(df)
df = df[df["input_size"] <= 32768]

g = sns.FacetGrid(
    df, 
    col="opcode", 
    col_wrap=4, 
    height=3, 
    sharex=False, 
    sharey=False
)

g.map_dataframe(
    sns.lineplot, 
    x="input_size", 
    y="instruction_delta_per_op",
    estimator="mean",
    errorbar="ci",
    err_style="band"
)

g.set_titles(col_template="{col_name}")
g.set_axis_labels("Input Size", "Instruction Cost")
g.figure.subplots_adjust(top=0.92)
plt.suptitle("Instruction Cost vs Input Size per Opcode")

plt.show()