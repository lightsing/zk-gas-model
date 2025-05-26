import pandas as pd
import scipy.stats as stats
import matplotlib.pyplot as plt
import seaborn as sns


df = pd.read_csv("results.csv")
df["instruction_delta_per_op"] = (df["exec_instruction_count"] - df["baseline_instruction_count"]) / df["repetition"]

# print("\nShapiro-Wilk 正态性检验（instruction_delta_per_op）：")
# for opcode, group in df.groupby("opcode"):
#     values = group["instruction_delta_per_op"]
#     if len(values) < 3:
#         print(f"{opcode}: 样本太少，跳过")
#         continue
#     stat, p = stats.shapiro(values)
#     result = "近似正态" if p >= 0.05 else "非正态"
#     print(f"{opcode}: stat={stat:.4f}, p={p:.4g} → {result}")

def filter_p5_p95(group):
    p5 = group["instruction_delta_per_op"].quantile(0.05)
    p95 = group["instruction_delta_per_op"].quantile(0.95)
    return group[(group["instruction_delta_per_op"] >= p5) & (group["instruction_delta_per_op"] <= p95)]

df_clipped = df.groupby("opcode", group_keys=False).apply(filter_p5_p95, include_groups=True)

g = sns.FacetGrid(df_clipped, col="opcode", col_wrap=8, height=2, sharex=False, sharey=False)
g.map(sns.histplot, "instruction_delta_per_op", bins=25, kde=True)
g.set_titles(col_template="{col_name}")
g.fig.subplots_adjust(top=0.92)
plt.show()
