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
    files = sorted(
        [os.path.join(directory, f) for f in os.listdir(directory) if f.endswith('.yml') or f.endswith(".yaml")])
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

    i = 0
    while i < len(sorted_data):
        current_entry = sorted_data[i].copy()

        j = i + 1
        while j < len(sorted_data) and current_entry['finsh'] > sorted_data[j]['start']:
            next_entry = sorted_data[j]

            # Only consider overlaps where 'include' or 'exclude' tags match
            if ('include' in current_entry and 'include' in next_entry and
                current_entry['include'] == next_entry['include']) or \
                    ('exclude' in current_entry and 'exclude' in next_entry and
                     current_entry['exclude'] == next_entry['exclude']):

                # If the next entry's commit_time is more recent, update the 'finsh' time of the current entry
                if next_entry['commit_time'] > current_entry['commit_time']:
                    current_entry['finsh'] = next_entry['start']
                else:
                    # the next item has a commit time after the current item in which case it can be ignored
                    sorted_data.pop(j)

            j += 1
        # remove commit time
        if current_entry:
            current_entry.pop('commit_time', None)
            resolved_data.append(current_entry)

        i += 1

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


def create_historical_data(directory):
    raw_data = load_yaml_files(directory)
    aggregated_data = aggregate_data(raw_data, 'changes')
    num_entries_unfiltered = len(aggregated_data)
    aggregated_data = [entry for entry in aggregated_data if 'start' in entry]

    # Ensuring all 'start' and 'finsh' values are datetime objects
    for entry in aggregated_data:
        if isinstance(entry.get('start'), str):
            entry['start'] = parse(entry['start'])
        if isinstance(entry.get('finsh'), str):
            entry['finsh'] = parse(entry['finsh'])

    unique_entries_dict = {}

    for entry in aggregated_data:
        # Creating a unique key for each entry using a subset of the fields
        entry_key = (entry['start'], entry['finsh'], entry['stage'], entry.get('include', ''), entry.get('exclude', ''))

        # If this is a newer version of an already seen entry, replace the older version
        if entry_key not in unique_entries_dict or entry['commit_time'] > unique_entries_dict[entry_key]['commit_time']:
            unique_entries_dict[entry_key] = entry

    unique_entries = list(unique_entries_dict.values())

    sorted_data = sorted(unique_entries, key=lambda x: (x['start'], -x['commit_time'].timestamp()))
    num_entries_filtered = len(sorted_data)
    diff = num_entries_unfiltered - num_entries_filtered
    # print(f"{diff} entries were incorrectly formatted/duplicates and dropped")
    resolved_data = resolve_conflicts(sorted_data)

    resolved_data.sort(key=lambda x: x['start'], reverse=True)
    overlaps = find_overlaps(resolved_data)
    print(f"There are/is {len(overlaps)} over(s)")
    return resolved_data


if __name__ == "__main__":
    data = create_historical_data("../test-files")
    print(data)
