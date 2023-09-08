import datetime
import os
import yaml
from dateutil.parser import parse
import logging

logging.basicConfig(level=logging.INFO,
                    format='%(asctime)s - %(levelname)s - %(message)s',
                    filename='aggregate_historical_data.log',
                    filemode='a')
logger = logging.getLogger()
NEW_DIRECTORY = "historical_versions"
AGGREGATED_FILE = "historical_data.yml"


def find_overlaps(sorted_data):
    overlaps = []
    for i in range(len(sorted_data)):
        for j in range(i + 1, len(sorted_data)):
            if sorted_data[i]['finsh'] > sorted_data[j]['start']:
                overlaps.append((sorted_data[i], sorted_data[j]))
                if sorted_data[j]['start'] > sorted_data[i]['finsh']:
                    break
    return overlaps


def find_erroneous_line(file):
    with open(file, 'r') as f:
        for i, line in enumerate(f, start=1):
            try:
                yaml.safe_load(line)
            except Exception as inner_e:
                print(f"Error on line {i}: {inner_e}")
                print(f"Line content: {line.strip()}")


def load_yaml_files(directory):
    num_file_import_errors = 0
    files = sorted([os.path.join(directory, f) for f in os.listdir(directory) if f.endswith('.yml')])
    loadshedding_data = []
    for file in files:
        try:
            with open(file, 'r') as f:
                date_time_str = file.split("_")[-1]
                date_time_str = date_time_str.split(".")[0]
                data = yaml.safe_load(f)
                for change in data["changes"]:
                    change['commit_time'] = datetime.datetime.fromisoformat(date_time_str)
                loadshedding_data.append(data)
        except Exception as e:
            num_file_import_errors += 1
            logger.exception(f"Error in file {file}")
            continue
    print(f"There were {num_file_import_errors} files that could not be imported. Check aggregate_historical_data.log"
          f" for details")
    return loadshedding_data


def resolve_conflicts(sorted_data):


    resolved_data = []

    # Logic to resolve conflicts goes here

    return resolved_data


def write_yaml(filtered_aggregated_data, file_path):
    with open(file_path, 'w') as f:
        yaml.safe_dump({'historical_data': filtered_aggregated_data}, f)


def aggregate_data(data_as_dict, key):
    aggregated_data = []
    for entry in data_as_dict:
        for entries in entry[key]:
            aggregated_data.append(entries)
    return aggregated_data


if __name__ == "__main__":
    raw_data = load_yaml_files(NEW_DIRECTORY)
    aggregated_data = aggregate_data(raw_data, 'changes')
    num_entries_unfiltered = len(aggregated_data)
    aggregated_data = [entry for entry in aggregated_data if 'start' in entry]

    # Ensuring all 'start' and 'finsh' values are datetime objects
    for entry in aggregated_data:
        if isinstance(entry.get('start'), str):
            entry['start'] = parse(entry['start'])
        if isinstance(entry.get('finsh'), str):
            entry['finsh'] = parse(entry['finsh'])

    sorted_data = sorted(aggregated_data, key=lambda x: x.get('start'))
    num_entries_filtered = len(sorted_data)
    diff = num_entries_unfiltered - num_entries_filtered
    print(f"{diff} entries were incorrectly formatted and dropped")
    total_overlaps = 0
    overlaps = find_overlaps(sorted_data)
