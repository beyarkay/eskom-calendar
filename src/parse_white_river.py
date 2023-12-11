from tabula.io import read_pdf
import pandas as pd
import requests
import warnings
import os
import numpy as np

warnings.filterwarnings("ignore")


def main():
    # Get the path which the pdf should be saved to
    path = "generated/white-river-extension-1.pdf"
    url = "https://klcbt.co.za/wp-content/uploads/2023/11/White-River-Loadshedding-Schedule-and-Blocks.pdf"
    if not os.path.exists(path):
        r = requests.get(url, stream=True)
        with open(path, "wb") as f:
            f.write(r.content)

    # Read in the pdf via tabula
    pdf = read_pdf(path, pages="all")[0]
    assert type(pdf) is pd.DataFrame
    assert pdf is not None
    # drop first row
    pdf = pdf.drop(0)
    # Rename columns to be 'stage', 1, 2, 3,4 ...., 31
    pdf.columns = ["start_time", "finsh_time",
                   "stage_range"] + list(range(1, 32))
    # For some reason, the last three rows got shifted to the left when reading
    # in the PDF. Unshift them:
    pdf.iloc[-3:, 2:] = pdf.iloc[-3:, :-2]
    pdf.iloc[-3:, :2] = np.nan
    # For absolutely no reason on God's green earth, there's something funky
    # that happens with the 15h-17h30 time slots. Here's a fix:
    pdf.loc[29, ['start_time', 'finsh_time']] = ['15:00', '17:30']
    pdf.loc[32, ['start_time', 'finsh_time']] = ['16:00', '17:30']
    # Forward fill the start and end times
    pdf[['start_time', 'finsh_time']] = pdf[[
        'start_time', 'finsh_time']].ffill()
    # Expand out the abbreviated stage ranges: "3-4" -> [3, 4]
    new_rows = []
    for _i, row in pdf.iterrows():
        stage_range_split = row['stage_range'].split('-')
        stages = range(int(stage_range_split[0]), int(
            stage_range_split[1]) + 1)
        for stage in stages:
            new_row = row.copy().drop('stage_range')
            new_row['stage'] = stage
            new_rows.append(new_row)
    pdf = pd.DataFrame(new_rows)
    # Pivot the dataframe to have a 'block' column
    df = pdf.melt(
        id_vars=['start_time', 'finsh_time', 'stage'],
        var_name='date_of_month',
        value_name='block',
    )

    # Stages are subsets of each other, so add in the extra rows so that stage
    # 2 is a superset of stage 1, stage 3 a superset of stage 2, etc
    # for stage in range(1, 8):
    #     this_stage = df[df['stage'] == stage].copy()
    #     this_stage.stage = stage + 1
    #     df = pd.concat((this_stage, df))

    for block in sorted(df["block"].unique()):
        block_df = df[df["block"] == block].sort_values(
            ["date_of_month", "start_time", "stage"]
        )
        block_number = int(block.replace("B", ""))
        path = f"generated/white-river-extension-1-block-{block_number}.csv"
        block_df[["date_of_month", "start_time", "finsh_time", "stage"]].to_csv(
            path, index=False
        )
        print("Saved to " + path)


if __name__ == "__main__":
    main()
