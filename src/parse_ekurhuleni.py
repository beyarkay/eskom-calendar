import numpy as np
import os
import pandas as pd
import requests
import tabula
import warnings

warnings.filterwarnings("ignore")


def main():
    # Get the path which the pdf should be saved to
    path = "generated/gauteng-ekurhuleni.pdf"
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

    for area in range(1, 32):
        pdf[area] = pd.to_numeric(pdf[area], downcast="integer")

    pdf["start_time"] = [starts[i // 8] for i in range(pdf.shape[0])]
    pdf["finsh_time"] = [finshs[i // 8] for i in range(pdf.shape[0])]

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

    # For some reason block 14 has 2h30 long loadshedding, whereas the rest of
    # ekurhuleni has 2h long loadshedding. This piece of code fixes that See
    # https://github.com/beyarkay/eskom-calendar/issues/55"
    to_alt = {
        "02:00": "02:30",
        "04:00": "04:30",
        "06:00": "06:30",
        "08:00": "08:30",
        "10:00": "10:30",
        "12:00": "12:30",
        "14:00": "14:30",
        "16:00": "16:30",
        "18:00": "18:30",
        "20:00": "20:30",
        "22:00": "22:30",
        "23:59": "00:30",
    }
    pdf["finsh_time"] = pdf.apply(
        lambda row: to_alt[row["finsh_time"]]
        if row["area"] == 14
        else row["finsh_time"],
        axis=1,
    )

    areas = []
    for area_idx in sorted(pdf["area"].unique()):
        area_df = pdf[pdf["area"] == area_idx].sort_values(
            ["date_of_month", "start_time", "stage"]
        )[["date_of_month", "start_time", "finsh_time", "stage"]]
        print(area_df[area_df.stage == 8].head())
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
                print(f"Combining `{prev.values}` and `{curr.values}`")
                prev["finsh_time"] = curr["finsh_time"]
            else:
                df2 = pd.concat((df2, curr.to_frame().T), ignore_index=True)
        area_df = df2.sort_values(["date_of_month", "start_time", "stage"])
        print(area_df[area_df.stage == 8].head())
        path = f"generated/gauteng-ekurhuleni-block-{int(area_idx)}.csv"
        area_df.to_csv(path, index=False)
        print("Saved to " + path)


if __name__ == "__main__":
    main()
