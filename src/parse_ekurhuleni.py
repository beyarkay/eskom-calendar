import numpy as np
import os
import pandas as pd
import requests
import sys
import tabula
import warnings

warnings.filterwarnings("ignore")


def main():
    # Get the path which the pdf should be saved to
    path = f"generated/gauteng-ekurhuleni.pdf"
    url = "https://www.ekurhuleni.gov.za/wp-content/uploads/2021/11/Ekurhuleni-Load-Shedding-Schedule.pdf"
    if not os.path.exists(path):
        r = requests.get(url, stream=True)
        with open(path, "wb") as f:
            f.write(r.content)

    # Read in the pdf via tabula
    pdf_pages = tabula.read_pdf(path, pages="all")
    # Use the column headers as a row
    pdf2 = pdf_pages[1].T.reset_index().T
    pdf2.iloc[1] = pd.concat((pdf2.iloc[1].iloc[1:], pd.Series([np.nan])))
    pdf2.iloc[0] = pd.concat((pdf2.iloc[0].iloc[1:], pd.Series([np.nan])))
    # Bunch of data cleaning
    pdf2.iloc[0] = pdf2.iloc[0].apply(
        lambda x: np.nan if "Unnamed" in str(x) else np.round(float(x))
    )
    for i in range(pdf2.shape[0]):
        while pd.isna(pdf2.iloc[i].iloc[-1]):
            pdf2.iloc[i] = pd.concat((pd.Series([np.nan]), pdf2.iloc[i].iloc[:-1]))

    # Get rid of unused columns
    pdf2 = pdf2.iloc[:, 5:]
    # Manually set the stage to 3
    pdf2.iloc[0, 0] = 3
    # Reset the columns
    pdf2.columns = ["stage"] + list(range(1, 32))
    pdf2["stage"] = pdf2["stage"].apply(lambda x: int(str(x)[0]))
    pdf2["start_time"] = "22:00"
    pdf2["finsh_time"] = "23:59"

    pdf = pdf_pages[0]
    # Set the columns to the first row
    pdf.columns = (
        list(pdf.iloc[0][:3].values) + [np.nan] * 6 + list(pdf.iloc[0][3:-6].values)
    )
    # remove the first row
    pdf = pdf.drop(0)

    # Shuffle things to be right-aligned
    for i in range(pdf.shape[0]):
        while pd.isna(pdf.iloc[i].iloc[-1]):
            pdf.iloc[i] = pd.concat((pd.Series([np.nan]), pdf.iloc[i].iloc[:-1]))

    # Drop unused columns
    pdf.columns = list(pdf.columns[:8]) + ["stage"] + list(pdf.columns[9:])
    cols = [c for c in pdf.columns if not pd.isna(c)]
    pdf = pdf[cols]

    # Get a nicely formatted start and finish
    pdf[["start_time", "finsh_time"]] = pdf["Start"].apply(
        lambda x: pd.Series([np.nan, np.nan] if pd.isna(x) else [x[:5], x[5:]])
    )

    pdf["stage"] = pdf["Stage"].apply(lambda x: x if pd.isna(x) else int(str(x)[0]))

    # We don't need these columns any more
    pdf = pdf.drop(["Start", "End", "Stage"], axis=1)

    # Rename columns to be 'stage', 1, 2, 3,4 ...., 31
    pdf.columns = ["stage"] + list(range(1, 32)) + ["start_time", "finsh_time"]

    # Join the two PDFs together
    pdf = pd.concat((pdf, pdf2))

    pdf["stage"] = list(range(1, 9)) * 12

    # Create some temporary arrays so that we can re-construct the start and
    # finish times associated with each row. They go 00:00-02:30 x8 times, then
    # 02:00-04:30 x8 times, and so on
    starts = [
        "00:00",
        "02:00",
        "04:00",
        "06:00",
        "08:00",
        "10:00",
        "12:00",
        "14:00",
        "16:00",
        "18:00",
        "20:00",
        "22:00",
    ]
    finshs = [
        "02:00",
        "04:00",
        "06:00",
        "08:00",
        "10:00",
        "12:00",
        "14:00",
        "16:00",
        "18:00",
        "20:00",
        "22:00",
        "23:59",
    ]

    pdf["start_time"] = [starts[i // 8] for i in range(pdf.shape[0])]
    pdf["finsh_time"] = [finshs[i // 8] for i in range(pdf.shape[0])]
    for area in range(1, 32):
        pdf[area] = pd.to_numeric(pdf[area], downcast="integer")

    # Stages are subsets of each other, so add in the extra rows so that stage
    # 2 is a superset of stage 1, stage 3 a superset of stage 2, etc
    for stage in range(1, 8):
        this_stage = pdf[pdf.stage == stage].copy()
        this_stage.stage = stage + 1
        pdf = pd.concat((this_stage, pdf))

    pdf = pdf.melt(
        id_vars=["stage", "start_time", "finsh_time"],
        value_name="area",
        var_name="date_of_month",
    ).sort_values(["stage", "date_of_month", "start_time"])

    areas = []
    for area_idx in sorted(pdf["area"].unique()):
        area_df = pdf[pdf["area"] == area_idx].sort_values(
            ["date_of_month", "start_time", "stage"]
        )
        path = f"generated/gauteng-ekurhuleni-block-{int(area_idx)}.csv"
        area_df[["date_of_month", "start_time", "finsh_time", "stage"]].to_csv(
            path, index=False
        )
        print("Saved to " + path)


if __name__ == "__main__":
    main()
