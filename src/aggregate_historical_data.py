import os
import yaml
from dateutil.parser import parse

NEW_DIRECTORY = "historical_versions"
AGGREGATED_FILE = "historical_data.yml"


def find_overlaps(data):
    # Sort the data based on start time
    sorted_data = sorted(data, key=lambda x: parse(x['start']))

    overlaps = []

    # Loop through sorted data
    for i in range(len(sorted_data) - 1):
        # If end time of current item is greater than start time of next item
        if parse(sorted_data[i]['finsh']) > parse(sorted_data[i+1]['start']):
            overlaps.append((sorted_data[i], sorted_data[i+1]))

    return overlaps


def load_yaml_files(directory):
    files = sorted([os.path.join(directory, f) for f in os.listdir(directory) if f.endswith('.yml')])
    loadshedding_data = []
    for file in files:
        with open(file, 'r') as f:
            loadshedding_data.append(yaml.safe_load(f))
    return loadshedding_data


def aggregate_data(loadshedding_raw_data):
    unfiltered_aggregated_data = []

    for d in loadshedding_raw_data:
        unfiltered_aggregated_data.extend(d.get('changes', []))

    return unfiltered_aggregated_data


def resolve_conflicts(aggregated_data):
    # Here, implement the logic to resolve conflicts
    # It will involve sorting the entries by 'start' date and identifying & resolving overlaps
    sorted_data = sorted(aggregated_data, key=lambda x: parse(x['start']))

    resolved_data = []

    # Logic to resolve conflicts goes here

    return resolved_data


def write_yaml(filtered_aggregated_data, file_path):
    with open(file_path, 'w') as f:
        yaml.safe_dump({'historical_data': filtered_aggregated_data}, f)


if __name__ == "__main__":
    # raw_data = load_yaml_files(NEW_DIRECTORY)
    # unfiltered_data = aggregate_data(raw_data)
    # filtered_data = resolve_conflicts(unfiltered_data)
    # write_yaml(filtered_data, AGGREGATED_FILE)
    sample_data = [
        {'start': '2023-09-04T16:00:00', 'finsh': '2023-09-04T20:00:00', 'stage': 5},
        {'start': '2023-09-04T19:00:00', 'finsh': '2023-09-04T22:00:00', 'stage': 3},
        {'start': '2023-09-05T05:00:00', 'finsh': '2023-09-05T10:00:00', 'stage': 6},
    ]

    sorted_data = sorted(sample_data, key=lambda x: parse(x['start']))

    for entry in sorted_data:
        print(entry)

    overlaps = find_overlaps(sample_data)
    for overlap in overlaps:
        print(f"Overlap between: {overlap[0]} and \n{overlap[1]}")