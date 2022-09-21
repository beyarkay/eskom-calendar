import tabula
import sys
import pandas as pd
import requests
import warnings
import os

warnings.filterwarnings("ignore")


def main():
    # Check that the usage is correct
    if len(sys.argv) != 3:
        print("Usage: `python3 parse_eskom.py <URL_TO_PDF> <WRITE_FILE_TO_HERE>")
        exit(1)
    # Get the path which the pdf should be saved to
    path = f"generated/{sys.argv[2]}.pdf"
    url = sys.argv[1]
    if not os.path.exists(path):
        r = requests.get(url, stream=True)
        with open(path, "wb") as f:
            f.write(r.content)

    # Read in the pdf via tabula
    pdf = tabula.read_pdf(path, pages=1)[0]
    # Drop the last few rows, they don't have anything but NaNs
    pdf.drop(range(13, 41), inplace=True)
    # Fix a bug that only occurs on the Kirkwood pdf
    if (
        url
        == "https://www.eskom.co.za/distribution/wp-content/uploads/2021/06/Kirkwood.pdf"
    ):
        pdf.loc[6, "Unnamed: 1"] = "10:00"
    # Fix a bug that only occurs on the MookgophongNaboomspruit_Town pdf
    if (
        url
        == "https://www.eskom.co.za/distribution/wp-content/uploads/2021/06/MookgophongNaboomspruit_Town.pdf"
    ):
        pdf.loc[8, "Unnamed: 1"] = "15:00"
    # Fix a bug that only occurs on the Umtata pdf
    if (
        url
        == "https://www.eskom.co.za/distribution/wp-content/uploads/2021/06/Umtata.pdf"
    ):
        pdf.loc[6, "Unnamed: 1"] = "11:00"
    # Create an empty dataframe
    df = pd.DataFrame()
    # Extract the start and finish times from the pdf
    df[["start", "finsh"]] = pdf.iloc[1:, 2:4]
    # Create a temporary column that contains all the load shedding stages
    # joined together
    pdf["vals"] = pdf.apply(
        lambda row: ",".join(str(v) for v in row.iloc[4:10]).replace(" ", ","), axis=1
    )
    # And populate the df with the load shedding stages on different dates
    df[list(range(1, 32))] = pdf.loc[1:, "vals"].apply(
        lambda x: pd.Series([int(float(v)) for v in x.split(",")])
    )
    # Set the start-finish times as the index
    df.set_index(["start", "finsh"], inplace=True)
    # Stack the dataframe to be one *long* Series, with an index
    # like (start, finish, date)
    df = df.stack()
    # Convert the index to columns
    df = df.reset_index()
    # Give the columns better names
    df.columns = pd.Index(["start_time", "finsh_time", "date_of_month", "stage"])

    to_add = []
    for i, row in df.iterrows():
        if row["stage"] == 0:
            continue
        for stage in range(row["stage"] + 1, 8 + 1):
            to_add.append(
                {
                    "start_time": row["start_time"],
                    "finsh_time": row["finsh_time"],
                    "date_of_month": row["date_of_month"],
                    "stage": stage,
                }
            )
    for d in to_add:
        print(f"adding to df: {d=}")
        df = df.append(d, ignore_index=True)

    assert not df.isna().any().any(), "DF has some NaN values"

    # We don't bother including stage == 0
    df = df[df["stage"] > 0]

    # Now the big task is to de-duplicate the stages. For example:
    # date_of_month start_time finsh_time  stage
    #             1      02:00      04:30      8
    #             1      04:00      06:30      8
    #             1      10:00      12:30      8
    #             1      12:00      14:30      8
    # The first two rows should really be one row like:
    # date_of_month start_time finsh_time  stage
    #             1      02:00      06:30      8
    # So this code combines them:
    df = df.sort_values(["stage", "date_of_month", "start_time"])
    df2 = pd.DataFrame(columns=df.columns)
    for i, curr in df.iterrows():
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
        starts_before_finish = pd.to_datetime(prev["start_time"]) < pd.to_datetime(
            curr["finsh_time"]
        )
        finishes_after_start = pd.to_datetime(prev["finsh_time"]) > pd.to_datetime(
            curr["start_time"]
        )
        if starts_before_finish and finishes_after_start:
            prev["finsh_time"] = curr["finsh_time"]
        else:
            df2 = pd.concat((df2, curr.to_frame().T), ignore_index=True)

    # Sort the values in the expected order
    df = df2.sort_values(["date_of_month", "start_time", "stage"])
    # Save to csv
    df[["date_of_month", "start_time", "finsh_time", "stage"]].to_csv(
        path.replace(".pdf", ".csv"), index=False
    )
    print("Saved to " + path.replace(".pdf", ".csv"))


if __name__ == "__main__":
    main()
