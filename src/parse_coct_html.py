import pandas as pd
import requests
from bs4 import BeautifulSoup
import html5lib
import os
import warnings

warnings.filterwarnings("ignore")


def main():

    last_area = 20
    dfs = []
    for file in [
        "html_sources/" + f
        for f in sorted(os.listdir("html_sources/"))
        if "city_of_cape_town" in f
    ]:
        stage = int(file[-6:-5])
        df = pd.read_html(file, flavor="html5lib")[0]
        df.replace(
            {
                "AREAS THAT WILL BE LOAD-SHED BETWEEN THE TIMES, TO THE LEFT, ON THE DAY OF THE MONTH, ABOVE": "",
                "DAYS OF THE MONTH": "dom",
                "FROM": "from",
                "TO": "to",
            },
            inplace=True,
        )
        df = df.drop([2])
        df["duration"] = df.apply(lambda x: f"{x.iloc[0]}-{x.iloc[1]}", axis=1)
        df = df.drop([0, 1], axis=1)
        for date_of_month in range(1, 31 + 1):
            df[str(date_of_month)] = df.iloc[:, (date_of_month - 1) % 16]
        df = df.drop(range(2, 18), axis=1)
        df = df.drop([0, 1])
        df = df.set_index("duration")
        df = df.stack().reset_index()
        df.columns = ["duration", "date_of_month", "areas"]
        # get a DF with `last_area` columns. df[:, 1] is True if area 1 has loadshedding at that time
        area_has_ls = df["areas"].apply(
            lambda x: pd.Series(
                [(v in [int(xi) for xi in x.split(",")]) for v in range(0, last_area)]
            )
        )
        df = pd.concat([df, area_has_ls], axis=1)
        df["start_time"] = df["duration"].apply(lambda x: x.split("-")[0][:5])
        df["finsh_time"] = df["duration"].apply(lambda x: x.split("-")[1][:5])
        df["stage"] = stage
        df = df.drop([0, "areas", "duration"], axis=1)
        df = df.set_index(
            ["stage", "date_of_month", "start_time", "finsh_time"]
        ).stack()
        df = df[df].reset_index().drop(0, axis=1)
        df.columns = ["stage", "date_of_month", "start_time", "finsh_time", "area"]
        dfs.append(df)

    all_dfs = pd.concat(dfs)
    areas = []
    for area_idx in sorted(all_dfs["area"].unique()):
        area_df = all_dfs[all_dfs["area"] == area_idx].sort_values(
            ["date_of_month", "start_time", "stage"]
        )

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

        path = f"generated/city-of-cape-town-area-{area_idx}.csv"
        area_df[["date_of_month", "start_time", "finsh_time", "stage"]].to_csv(
            path, index=False
        )
        print("Saved to " + path)


if __name__ == "__main__":
    main()
