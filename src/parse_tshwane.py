from bs4 import BeautifulSoup
import html5lib
import os
import pandas as pd
import requests
import ssl
import warnings

warnings.filterwarnings("ignore")


r = requests.get(
    "https://www.tshwane.gov.za/sites/Departments/Public-works-and-infrastructure/Pages/Load-Shedding.aspx",
    verify=ssl.CERT_NONE,
)
soup = BeautifulSoup(r.text, "html.parser")
selector = "div#WebPartWPQ3>div.ms-rtestate-field>table>tbody>tr>td>table"
table = soup.select_one(selector)
df = pd.read_html(str(table))[0]
# Drop useless columns
df = df.drop([0, 1, 2])
df.columns = ["start_time", "finsh_time", "stage"] + list(range(1, 32))
# Format the stage properly
df["stage"] = df["stage"].apply(lambda x: int(x[-1]))


# Melt the df to be in the format:
# stage start_time finsh_time date_of_month area
#     1      00:00      02:30             1    1
#     1      02:00      04:30             1    2
#     1      04:00      06:30             1    3

df = df.melt(
    id_vars=["stage", "start_time", "finsh_time"],
    value_name="area",
    var_name="date_of_month",
)

# Loadshedding stages are subsets of each other, so go through each stage and
# add the stages above it.
for stage in range(1, 8):
    this_stage = df[df.stage == stage].copy()
    this_stage.stage = stage + 1
    df = pd.concat((this_stage, df))

for area_idx in sorted(df["area"].unique()):
    area_df = df[df["area"] == area_idx].sort_values(
        ["date_of_month", "start_time", "stage"]
    )
    path = f"generated/gauteng-tshwane-group-{int(area_idx)}.csv"

    # Now the big task is to de-duplicate the stages. For example:
    # date_of_month start_time finsh_time  stage
    #             1      02:00      04:30      8
    #             1      04:00      06:30      8
    # The first two rows should really be one row like:
    # date_of_month start_time finsh_time  stage
    #             1      02:00      06:30      8
    # So this code combines them:
    area_df = area_df.sort_values(["stage", "date_of_month", "start_time"])
    df2 = pd.DataFrame(columns=area_df.columns)
    for i, curr in area_df.iterrows():
        # the first row always gets added
        if len(df2) == 0:
            df2 = pd.concat((df2, curr.to_frame().T), ignore_index=True)
            continue
        prev = df2.iloc[-1]
        # Don't attempt to combine rows of different stages
        if prev["stage"] != curr["stage"]:
            df2 = pd.concat((df2, curr.to_frame().T), ignore_index=True)
            continue
        # If this row and the previous row overlap in their times => combine them
        starts_before_finish = pd.to_datetime(prev["start_time"]) <= pd.to_datetime(
            curr["finsh_time"]
        )
        finishes_after_start = pd.to_datetime(prev["finsh_time"]) >= pd.to_datetime(
            curr["start_time"]
        )
        if starts_before_finish and finishes_after_start:
            prev["finsh_time"] = curr["finsh_time"]
        else:
            df2 = pd.concat((df2, curr.to_frame().T), ignore_index=True)
    area_df = df2.sort_values(["date_of_month", "start_time", "stage"])

    area_df[["date_of_month", "start_time", "finsh_time", "stage"]].to_csv(
        path, index=False
    )
    print("Saved CSV schedule to " + path)
