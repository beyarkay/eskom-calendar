import os
import shutil
import yaml
from git import Repo
from datetime import datetime
import logging

# Configure logging
logging.basicConfig(filename='file_versions_log.log', level=logging.INFO,
                    format='%(asctime)s - %(levelname)s - %(message)s')
REPO_URL = "https://github.com/beyarkay/eskom-calendar.git"
BRANCH_NAME = "main"
FILE_NAME = "manually_specified.yaml"
NEW_DIRECTORY = "historical_versions"
TEMP_PATH = "eskom-calendar-temp"


def get_file_history(repo_name, branch_name, file_name):
    """Get commit hashes where the file changed."""
    git_commits = list(repo_name.iter_commits(branch_name, paths=file_name))
    return git_commits


def copy_file_versions(commit_hashes, file_name, new_directory):
    """Copy versions of the file to a new directory."""
    if not os.path.exists(new_directory):
        os.makedirs(new_directory)

    for i, commit in enumerate(commit_hashes):
        try:
            # Get the file content at this commit
            file_content = (commit.tree / file_name).data_stream.read()
        except Exception as e:
            logging.error(f"Failed to read file content at commit {commit.hexsha} due to: {str(e)}")
            continue

        try:
            # Create a new file with the content at this commit with commit hash and time
            commit_time = datetime.fromtimestamp(commit.committed_date).isoformat()
            new_file_name = os.path.join(new_directory,
                                         f"{os.path.splitext(file_name)[0]}_{commit.hexsha}_{commit_time}.yml")
            with open(new_file_name, "wb") as f:
                f.write(file_content)
        except Exception as e:
            logging.error(f"Failed to write file content due to: {str(e)}")
            continue

        logging.info(f"Successfully copied version {i + 1}/{len(commit_hashes)} to {new_file_name}")


if __name__ == "__main__":
    repo = ''
    try:
        repo = Repo.clone_from(REPO_URL, TEMP_PATH, branch=BRANCH_NAME)
    except Exception as e:
        if os.path.exists(TEMP_PATH):
            shutil.rmtree(TEMP_PATH)
        print(f"Cannot clone {REPO_URL} due to {e}")
        exit(1)

    try:
        # Get the commits where the file changed
        commits = get_file_history(repo, BRANCH_NAME, FILE_NAME)
        copy_file_versions(commits, FILE_NAME, NEW_DIRECTORY)
        logging.info(f"Copied all versions of {FILE_NAME} to {NEW_DIRECTORY}")
    except Exception as e:
        logging.critical(f"Script failed due to: {str(e)}")
    print(f"Copied all versions of {FILE_NAME} to {NEW_DIRECTORY}")

    # Remove the temporary cloned repository
    if os.path.exists(TEMP_PATH):
        try:
            shutil.rmtree(TEMP_PATH)
            print(f"Directory {TEMP_PATH} has been removed successfully")
        except OSError as error:
            print(error)
            print(f"Directory {TEMP_PATH} can not be removed")
