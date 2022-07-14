import tabula
import sys
import pandas as pd
import requests

def main():
    # Check that the usage is correct
    if len(sys.argv) != 3:
        print("Usage: `python3 parse_pdf.py <URL_TO_PDF> <WRITE_FILE_TO_HERE>")
        exit(1)
    # Get the path which the pdf should be saved to
    path = f'generated/{sys.argv[2]}.pdf'
    url = sys.argv[1]
    r = requests.get(url, stream=True)

    with open(path, 'wb') as f:
        f.write(r.content)

    # Read in the pdf via tabula
    pdf = tabula.read_pdf(path, pages=1)[0]
    # Drop the last few rows, they don't have anything but NaNs
    pdf.drop(range(13, 41), inplace=True)
    # Create an empty dataframe
    df = pd.DataFrame()
    # Extract the start and finish times from the pdf
    df[["start", "finsh"]] = pdf.iloc[1:, 2:4]
    # Create a temporary column that contains all the load shedding stages
    # joined together
    pdf['vals'] = pdf.apply(lambda row: ','.join(str(v) for v in row.iloc[4:10]).replace(' ', ','), axis=1)
    # And populate the df with the load shedding stages on different dates
    df[list(range(1, 32))] = pdf.loc[1:, 'vals'].apply(lambda x: pd.Series([int(float(v)) for v in x.split(',')]))
    # Set the start-finish times as the index
    df.set_index(['start', 'finsh'], inplace=True)
    # Stack the dataframe to be one *long* Series, with an index 
    # like (start, finish, date)
    df = df.stack()
    # Convert the index to columns
    df = df.reset_index()
    # Give the columns better names
    df.columns = pd.Index(['start_time', 'finsh_time', 'date_of_month', 'stage'])

    # Save to csv
    df.to_csv(path.replace(".pdf", ".csv"), index=False)
    print("Saved to " + path.replace(".pdf", ".csv"))

if __name__ == "__main__":
    main()
