from tabula.io import read_pdf
import numpy as np
import os
import pandas as pd
import requests
import warnings

warnings.filterwarnings("ignore")


def main():

    path = "generated/buffalo-city.pdf"
    url = "https://www.buffalocity.gov.za/CM/uploads/documents/20201001011578662845LoadsheddingInformationPackV5.pdf"  # noqa: E501

    pdf_pages = dl_pdf(path, url)
    first_half_1_4 = pdf_pages[6]
    secnd_half_1_4 = pdf_pages[7]

    first_half_5_8 = pdf_pages[8]
    secnd_half_5_8 = pdf_pages[9]

    # Some of the rows in first_half_1_4 are incorrectly NaN. Remove them
    df_14 = first_half_1_4[~first_half_1_4["Stage"].isna()].reset_index(drop=True)
    df_14 = pd.concat((df_14, secnd_half_1_4), axis=1)
    # Some of the rows in first_half_5_8 are incorrectly NaN. Remove them.
    df_58 = first_half_5_8[~first_half_5_8["Stage"].isna()].reset_index(drop=True)
    df_58 = pd.concat((df_58.iloc[4:], secnd_half_5_8.iloc[4:]), axis=1)
    # Concat the two DFs into one
    df = pd.concat((df_14, df_58))

    # Remove all the "Block " text
    df = df.replace(to_replace=".*Block (.*)", value="\\1", regex=True)
    # # Replace "15 & 10" with "15,10"
    df = df.replace(to_replace="(\d+)\s?\&\s?(\d+)", value="\\1,\\2", regex=True)
    # # Replace "Stage X" with "X"
    df = df.replace(to_replace="Stage\s?(\d+)", value="\\1", regex=True)
    # We don't need the Time column anymore
    df = df.drop(columns="Time")
    # Lowercase the Stage column
    df = df.rename(columns={"Stage": "stage"})
    # Reorder the columns
    df.columns = ["stage"] + list(range(1, 32))

    # Create start and finish times for the 1-4 and 5-8 portions
    starts14 = [f"{i % 24:0>2}:00" for i in range(0, 24, 3)]
    finshs14 = [f"{i % 24:0>2}:00" for i in range(3, 25, 3)]
    starts58 = [f"{i % 24:0>2}:00" for i in range(2, 24, 3)]
    finshs58 = [f"{i % 24:0>2}:00" for i in range(5, 27, 3)]
    # Label the start and finsh times
    df["start_time"] = np.concatenate((np.repeat(starts14, 4), np.repeat(starts58, 4)))
    df["finsh_time"] = np.concatenate((np.repeat(finshs14, 4), np.repeat(finshs58, 4)))

    # Reorder the columns
    df = df[["start_time", "finsh_time", "stage"] + list(range(1, 32))].reset_index(
        drop=True
    )

    # Melt the DF from
    # `stage,start,finsh,1st,2nd,3rd,4th,...`
    # to
    # `stage,start,finsh,date_of_month`
    df = df.melt(
        id_vars=["start_time", "finsh_time", "stage"],
        value_vars=list(range(1, 32)),
        value_name="block",
        var_name="date_of_month",
    )

    # Extract each block's information, and write it to a PDF
    NUM_BLOCKS = 19
    dfs = {}
    for block in range(1, NUM_BLOCKS + 1):
        dfs[block] = df.loc[
            df["block"].str.split(",").apply(lambda x: str(block) in x),
            ["date_of_month", "start_time", "finsh_time", "stage"],
        ].sort_values(["stage", "date_of_month", "start_time"])

        dfs[block].to_csv(f"generated/buffalo-city-block-{block}.csv", index=False)


def dl_pdf(path, url):
    if not os.path.exists(path):
        r = requests.get(url, stream=True)
        with open(path, "wb") as f:
            f.write(r.content)
    # Read in the pdf via tabula
    pdf_pages = read_pdf(path, pages="all")
    return pdf_pages


if __name__ == "__main__":
    main()
