import sys

import seaborn as sns
import matplotlib.pyplot as plt
from sklearn.linear_model import LinearRegression

from process import *

try:
    name = sys.argv[1]
except:
    print("usage: <name> [input]")

try:
    file = sys.argv[2]
except IndexError:
    file = "results.csv"

df = process_simple(pd.read_csv(file))
df = clip_p5_p95(df)
df = df[df["name"] == name]
df_grouped = df.groupby("input_size")["instruction_delta_per_op"]
agged = df_grouped.agg(agg_mode).reset_index()

x = agged["input_size"].values.reshape(-1, 1)
y = agged["instruction_delta_per_op"].values

model = LinearRegression()
model.fit(x, y)  # 线性回归
y_fit = model.predict(df["input_size"].values.reshape(-1, 1))
slope = model.coef_[0]
intercept = model.intercept_
r2 = model.score(x, y)
print(f"y = {slope}x + {intercept}")

plt.figure(figsize=(10, 6))

sns.lineplot(
    data=df,
    x="input_size",
    y="instruction_delta_per_op",
    estimator="mean",
    errorbar="ci",
    err_style="band",
    color="blue"
)

plt.plot(df["input_size"], y_fit, color="red", label=f"Linear Fit: y = {slope:.2f}x + {intercept:.2f}", linewidth=2)
plt.legend()
plt.grid(True)

plt.tight_layout()
plt.show()
