import requests
import yaml
import re
import time
import datetime
import pandas as pd

pd.set_option("display.max_columns", 10)
pd.set_option("display.max_rows", 20)
pd.set_option("display.width", 100)

provinces = {
    "EC": "https://www.eskom.co.za/distribution/wp-content/uploads/2022/09/EasternCape_LS.xlsx",
    "FS": "https://www.eskom.co.za/distribution/wp-content/uploads/2022/09/FreeState_LS.xlsx",
    "GP": "https://www.eskom.co.za/distribution/wp-content/uploads/2022/09/Gauteng_LS.xlsx",
    "KZN": "https://www.eskom.co.za/distribution/wp-content/uploads/2022/09/KwaZulu-Natal_LS.xlsx",
    "LP": "https://www.eskom.co.za/distribution/wp-content/uploads/2022/09/Limpopo_LS.xlsx",
    "NC": "https://www.eskom.co.za/distribution/wp-content/uploads/2022/09/NorthernCape_LS.xlsx",
    "NW": "https://www.eskom.co.za/distribution/wp-content/uploads/2022/09/NorthWest_LS.xlsx",
    "WC": "https://www.eskom.co.za/distribution/wp-content/uploads/2022/09/WesternCape_LS.xlsx",
}

province_map = {
    "EC": "eastern-cape",
    "FS": "free-state",
    "GP": "gauteng",
    "KZN": "kwazulu-natal",
    "LP": "limpopo",
    "NC": "northern-cape",
    "NW": "north-west",
    "WC": "western-cape",
}


def download_excel(province_url):
    print(f"Downloading excel file {province_url}")
    storage_options = {"User-Agent": "Mozilla/5.0"}
    return pd.read_excel(province_url, None, storage_options=storage_options)


def excel_to_schedule(sheets):
    print("Converting excel to schedules")
    sched = sheets["Schedule"].copy()  # Extract the schedule from the spreadsheet
    sched = sched.iloc[14:110, :]  # Only get info we care about
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

    # LoadShedding officially only ends at HH:30, not at HH:00
    sched["finsh"] = sched["finsh"].apply(lambda t: t.replace(minute=30))
    # Convert time objects to strings
    sched[["start", "finsh"]] = sched[["start", "finsh"]].applymap(
        lambda t: t.strftime("%H:%M")
    )

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
            # If this row and the previous row overlap in their times => combine them
            starts_before_finish = pd.to_datetime(prev["start_time"]) <= pd.to_datetime(
                curr["finsh_time"]
            )
            finishes_after_start = pd.to_datetime(prev["finsh_time"]) >= pd.to_datetime(
                curr["start_time"]
            )
            if starts_before_finish and finishes_after_start:
                # print(f"Combining `{prev.values}` and `{curr.values}`")
                prev["finsh_time"] = curr["finsh_time"]
            else:
                df2 = pd.concat((df2, curr.to_frame().T), ignore_index=True)
        # area_df = df2.sort_values(["date_of_month", "start_time", "stage"])
        areas[area_idx] = df2.sort_values(["date_of_month", "start_time", "stage"])
    return areas


def make_odd_even_areas(areas):
    """This method takes each block and makes both the odd and even starts.
    Eskom for some reason uses U to mean odd loadshedding starts (01:00,
    03:00, etc) and E to mean even loadshedding starts (00:00, 02:00, etc).
    """
    print("Making odd and even areas")
    new_areas = {}
    for block, df in areas.items():
        new_areas[f"{block}-even"] = df.copy()
        add_one_hour = lambda x: (x + datetime.timedelta(hours=1)).strftime("%H:%M")
        df["start_time"] = pd.to_datetime(df["start_time"]).apply(add_one_hour)
        df["finsh_time"] = pd.to_datetime(df["finsh_time"]).apply(add_one_hour)
        new_areas[f"{block}-odd"] = df.copy()
    return new_areas


def save_areas(areas):
    for idx, area_df in areas.items():
        area_df[["date_of_month", "start_time", "finsh_time", "stage"]].sort_values(
            ["date_of_month", "stage", "start_time"]
        ).to_csv(f"generated/eskom-direct-{idx}.csv", index=False)


def main():
    sheets = download_excel(provinces["WC"])
    sched = excel_to_schedule(sheets)
    df = pivot_schedule(sched)
    df = subset_stages(df)
    areas = filter_duplicates(df)
    areas = make_odd_even_areas(areas)
    save_areas(areas)


all_suburbs = []
for acronym, url in provinces.items():
    sheets = download_excel(provinces[acronym])
    # Get a list of the suburbs
    suburbs = sheets["SP_List"]
    # Rename some columns
    suburbs = suburbs.rename(
        columns={
            "MP_NAME": "municipality",
            "SP_NAME": "suburb",
            "BLOCK": "block",
            "TYPE": "type",
        }
    )[["municipality", "suburb", "block", "type"]]
    # Normalize all municipalities
    suburbs["municipality"] = suburbs["municipality"].apply(
        lambda x: x.lower().replace(" ", "-")
    )
    # Normalize all suburbs, removing the block suffix "Aardoff (2)" => "aardoff"
    suburbs["suburb"] = suburbs["suburb"].apply(
        lambda x: re.sub(r"\s\(\d+\)", "", x.lower()).replace(" ", "-")
    )
    suburbs["province"] = province_map[acronym]
    all_suburbs.append((suburbs))
suburbs = pd.concat(all_suburbs)

# PyYaml isn't flexible enough to have some items be block-format and others be
# flow-format. So just write to a tmp file and copy into the correct file. It only
# has to be done once.

text = []
for name, group in suburbs.groupby(["block", "type"]):
    odd_or_even = "odd" if name[1].endswith("U") else "even"
    text.append(
        f"""\
  - calendar_name: generated/eskom-direct-{name[0]}-{odd_or_even}.ics
    provider: eskom
    source: https://www.eskom.co.za/distribution/wp-content/uploads/2022/09/WesternCape_LS.xlsx
    source_info: https://www.eskom.co.za/distribution/customer-service/outages/downloadable-loadshedding-spreadsheets-for-eskom-customers/
    areas:"""
    )
    for name2, group2 in group.groupby(["province", "municipality"]):
        names = ",".join([f'"{s}"' for s in group2["suburb"].unique()])
        text.append(
            f"""\
    - province: {name2[0]}
      municipality: "{name2[1]}"
      name: [{names}]"""
        )
#         break
# print("\n".join(text))

with open("tmp.yaml", "w") as f:
    f.write("\n".join(text))

if __name__ == "__main__":
    main()
