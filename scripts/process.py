import numpy as np
import pandas as pd

def process_simple(df: pd.DataFrame) -> pd.DataFrame:
    delta = df["exec_instruction_count"] - df["baseline_instruction_count"]
    if "instruction_count_consumes_by_other_estimated" in df.columns:
        delta -= df["instruction_count_consumes_by_other_estimated"]
    df["instruction_delta_per_op"] = delta / df["repetition"]
    return df

def clip_p5_p95(df: pd.DataFrame) -> pd.DataFrame:
    def filter_p5_p95(group):
        p5 = group["instruction_delta_per_op"].quantile(0.05)
        p95 = group["instruction_delta_per_op"].quantile(0.95)
        return group[group["instruction_delta_per_op"].between(p5,p95)]
    df_grouped = df.groupby("name", group_keys=True)
    df_clipped = df_grouped.apply(filter_p5_p95, include_groups=False)
    df_clipped = df_clipped.reset_index()
    return df_clipped

def auto_bin_count(data: pd.Series, method="fd"):
    data = data.dropna()
    if method == "sturges":
        return int(np.ceil(np.log2(len(data))) + 1)
    elif method == "sqrt":
        return int(np.ceil(np.sqrt(len(data))))
    elif method == "fd":
        q75, q25 = np.percentile(data, [75, 25])
        iqr = q75 - q25
        bin_width = 2 * iqr / (len(data) ** (1/3))
        data_range = data.max() - data.min()
        return max(1, int(np.ceil(data_range / bin_width)))
    else:
        return 25  # fallback

def binned_mode(series: pd.Series, bins=25):
    counts, edges = np.histogram(series, bins=bins)
    max_bin = np.argmax(counts)
    bin_center = (edges[max_bin] + edges[max_bin + 1]) / 2
    return bin_center

def agg_mode(data: pd.Series):
    bins = auto_bin_count(data)
    mode = binned_mode(data, bins=bins)
    return mode
