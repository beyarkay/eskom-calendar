import tabula
import sys
import pandas as pd
import requests
import warnings
import os

warnings.filterwarnings("ignore")


def main():
    # Get the path which the pdf should be saved to
    path = f"generated/citypower.pdf"
    url = "https://www.citypower.co.za/customers/Load%20Shedding%20Related%20Documents/City%20Power%20load%20shedding%20schedule%20February%202021.pdf"
    if not os.path.exists(path):
        r = requests.get(url, stream=True)
        with open(path, "wb") as f:
            f.write(r.content)

    # Read in the pdf via tabula
    pdf_pages = tabula.read_pdf(path, pages="all")
    pdf = pd.concat(pdf_pages)
    # Drop unused columns
    pdf = pdf.drop(
        ["City Power Load Shedding Schedule February 2021", "Unnamed: 0"], axis=1
    )
    # Rename columns to be 'stage', 1, 2, 3,4 ...., 31
    pdf.columns = ["stage"] + list(range(1, 32))
    # The first two rows don't have anything useful in them
    pdf = pdf.drop([0, 1])
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
        "02:30",
        "04:30",
        "06:30",
        "08:30",
        "10:30",
        "12:30",
        "14:30",
        "16:30",
        "18:30",
        "20:30",
        "22:30",
        "00:30",
    ]
    durations = [([f"{start}-{finish}"] * 8) for start, finish in zip(starts, finshs)]
    # Assign durations for each row
    pdf["duration"] = [durations[i // 8][i % 8] for i in range(pdf.shape[0])]
    # Reshape and rename the pdf to be like:
    #    duration stage  date_of_month  area
    # 00:00-02:30     1              1   1.0
    # 00:00-02:30     1              2  13.0
    # 00:00-02:30     1              3   9.0
    pdf = pdf.set_index(["duration", "stage"]).stack().reset_index()
    pdf.columns = ["duration", "stage", "date_of_month", "area"]

    pdf["stage"] = pd.to_numeric(pdf.stage)
    # Stages are subsets of each other, so add in the extra rows so that stage
    # 2 is a superset of stage 1, stage 3 a superset of stage 2, etc
    for stage in range(1, 8):
        this_stage = pdf[pdf.stage == stage].copy()
        this_stage.stage = stage + 1
        pdf = pd.concat((this_stage, pdf))

    max_area_code = int(pdf["area"].max())
    pdf["start_time"] = pdf["duration"].apply(lambda x: x.split("-")[0][:5])
    pdf["finsh_time"] = pdf["duration"].apply(lambda x: x.split("-")[1][:5])
    areas = []
    for area_idx in sorted(pdf["area"].unique()):
        area_df = pdf[pdf["area"] == area_idx].sort_values(
            ["date_of_month", "start_time", "stage"]
        )
        path = f"generated/city-power-{int(area_idx)}.csv"
        area_df[["date_of_month", "start_time", "finsh_time", "stage"]].to_csv(
            path, index=False
        )
        print("Saved to " + path)


if __name__ == "__main__":
    main()
