import sys

import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns

from process import *

try:
    file = sys.argv[1]
except IndexError:
    file = "results.csv"

df = process_simple(pd.read_csv(file))
df = clip_p5_p95(df)

# g = sns.FacetGrid(df, col="name", col_wrap=5, height=2, sharex=False, sharey=False)
# g.map(sns.histplot, "instruction_delta_per_op", bins=25, kde=True)
# g.set_titles(col_template="{col_name}")
# g.figure.subplots_adjust(top=0.92)
# plt.show()

def draw_distribution_with_stats(data, color=None, **kwargs):
    ax = plt.gca()
    sns.histplot(data["instruction_delta_per_op"], bins=25, kde=True, ax=ax, color=color)

    mean = data["instruction_delta_per_op"].mean()
    median = data["instruction_delta_per_op"].median()
    # mode_exact = data["instruction_delta_per_op"].mode()
    bins = auto_bin_count(data["instruction_delta_per_op"])
    mode_binned = binned_mode(data["instruction_delta_per_op"], bins=bins)

    ax.axvline(mean, color="blue", linestyle="--", linewidth=1.2, label="mean")
    ax.axvline(median, color="orange", linestyle="-.", linewidth=1.2, label="median")
    ax.axvline(mode_binned, color="green", linestyle=":", label="binned mode")
    # if not mode_exact.empty:
    #     ax.axvline(mode_exact.iloc[0], color="red", linestyle="--", label="exact mode")

    if "legend_drawn" not in kwargs:
        ax.legend(loc="upper right", fontsize=7)

g = sns.FacetGrid(df, col="name", col_wrap=5, height=2, sharex=False, sharey=False)
g.map_dataframe(draw_distribution_with_stats)

g.set_titles(col_template="{col_name}")
g.figure.subplots_adjust(top=0.92)
plt.show()
