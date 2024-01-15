import requests
import yaml
import re
import time
import datetime
import pandas as pd

pd.set_option("display.max_columns", 34)
pd.set_option("display.max_rows", 20)
pd.set_option("display.width", 100)

def download_excel(url):
    print(f"Downloading excel file {url}")
    storage_options = {"User-Agent": "Mozilla/5.0"}
    return pd.read_excel(url, None, storage_options=storage_options)


def excel_to_schedule(sheets):
    print("Converting excel to schedules")
    sched = sheets["Schedule"].copy()  # Extract the schedule from the spreadsheet

    sched = sched.iloc[2:110, :]  # Only get info we care about
    # Rename the columns to something reasonable
    sched.columns = ["start", "finsh", "stage"] + list(range(1, 32))
    # Replace a datetime mistake
    sched.loc[
        sched["finsh"] == datetime.datetime(1900, 1, 1, 0, 0), "finsh"
    ] = datetime.time(0, 0)
    # Forward-fill all the start/finsh times
    sched[["start", "finsh"]] = sched[["start", "finsh"]].ffill()
    # Convert non-numeric columns
    sched["stage"] = pd.to_numeric(sched.stage)
    sched[list(range(1, 32))] = sched[list(range(1, 32))].astype(int)

    return sched


def pivot_schedule(sched):
    print("Pivoting schedules")
    # Melt the schedule from wide to long format, with columns: [start, finsh,
    # stage, date_of_month, area]
    df = sched.melt(
        id_vars=["start", "finsh", "stage"],
        value_vars=list(range(1, 32)),
        value_name="area",
        var_name="date_of_month",
    )
    # Convert the dates from strings to numbers
    df["date_of_month"] = pd.to_numeric(df["date_of_month"])
    df = df.rename(columns={"start": "start_time", "finsh": "finsh_time"})
    return df


def subset_stages(df):
    for stage in range(1, 8):
        this_stage = df[df.stage == stage].copy()
        this_stage.stage = stage + 1
        df = pd.concat((this_stage, df))
    return df


def filter_duplicates(df):
    print("Filtering duplicates")
    areas = {}
    for area_idx in sorted(df["area"].unique()):
        print(f"Processing area {area_idx}")
        area_df = df[df["area"] == area_idx].sort_values(
            ["date_of_month", "start_time", "stage"]
        )

        # Combine loadshedding like 02:00-04:30 and 04:00-06:30 into one
        # loadshedding like 02:00-06:30
        area_df = area_df.sort_values(["stage", "date_of_month", "start_time"])
        df2 = pd.DataFrame(columns=area_df.columns)
        for i, curr in area_df.iterrows():
            # the first row always gets added
            if len(df2) == 0:
                df2 = pd.concat((df2, curr.to_frame().T), ignore_index=True)
                continue
            prev = df2.iloc[-1]
            # Don't attempt to combine rows of different stages or different
            # dates
            if (
                prev["stage"] != curr["stage"]
                or prev["date_of_month"] != curr["date_of_month"]
            ):
                df2 = pd.concat((df2, curr.to_frame().T), ignore_index=True)
                continue

            starts_before_finish = prev["start_time"] <= curr["finsh_time"]
            finishes_after_start = prev["finsh_time"] >= curr["start_time"]


            if starts_before_finish and finishes_after_start:
                # print(f"Combining `{prev.values}` and `{curr.values}`")
                prev["finsh_time"] = curr["finsh_time"]
            else:
                df2 = pd.concat((df2, curr.to_frame().T), ignore_index=True)
        areas[area_idx] = df2.sort_values(["date_of_month", "start_time", "stage"])
    return areas


def save_areas(areas):
    for idx, area_df in areas.items():
        area_df[["date_of_month", "start_time", "finsh_time", "stage"]].sort_values(
            ["date_of_month", "stage", "start_time"]
        ).to_csv(f"generated/city-power-{idx}.csv", index=False)


def main():
    url = "https://www.citypower.co.za/customers/Load%20Shedding%20Related%20Documents/City%20Power%202%20hour%20Stage%208%20Load%20Shedding%20Schedule%20Vers%206%2004%20Dec%202023.xlsx"
    sheet = download_excel(url)
    sched = excel_to_schedule(sheet)
    df = pivot_schedule(sched)
    df = subset_stages(df)
    areas = filter_duplicates(df)
    save_areas(areas)

if __name__ == "__main__":
    main()
