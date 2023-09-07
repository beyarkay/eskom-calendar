import os
import shutil
from git import Repo

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

    for commit in commit_hashes:
        # Get the file content at this commit
        file_content = (commit.tree / file_name).data_stream.read()

        # Create a new file with the content at this commit with commit hash appended
        new_file_name = os.path.join(new_directory, f"{os.path.splitext(file_name)[0]}_{commit.hexsha}.yml")
        with open(new_file_name, "wb") as f:
            f.write(file_content)


if __name__ == "__main__":

    # Clone the repository
    repo = Repo.clone_from(REPO_URL, TEMP_PATH, branch=BRANCH_NAME)

    # Get the commits where the file changed
    commits = get_file_history(repo, BRANCH_NAME, FILE_NAME)
    print(commits)
    # Copy versions of the file to a new directory
    copy_file_versions(commits, FILE_NAME, NEW_DIRECTORY)
    print(f"Copied all versions of {FILE_NAME} to {NEW_DIRECTORY}")

    # Remove the temporary cloned repository
    if os.path.exists(TEMP_PATH):
        try:
            shutil.rmtree(TEMP_PATH)
            print(f"Directory {TEMP_PATH} has been removed successfully")
        except OSError as error:
            print(error)
            print(f"Directory {TEMP_PATH} can not be removed")