"""Get tweets, try parse them as loadshedding info, and open an appropriate PR
in GitHub.

Okay this script is a bit all over the place because it uses two APIs, but it's
divided into some very discrete steps:

1. Get the latest tweets from @CityofCT
2. Try to parse all of those tweets as loadshedding tweets (they've got a
   very specific format)
3. Take the most recent of the parse-able loadshedding tweets, and convert it's
   content into a list of dicts, each dict representing one loadshedding
   change.
4. Create a new branch on GitHub based off of the `main` branch
5. Edit that branch so that the `manually_specified` file includes the latest
   tweet's updates.
6. Open a nicely formatted pull request to merge the new branch into `main`.
   Note that opening this PR will automatically trigger the tests to run.
7. If the tests pass, then Boyd will come along and merge the branch into
   `main`.
"""
import datetime
import base64
import json
import os
import re
import requests
import yaml


PRELUDE = """# How to edit this file:
# You should add items to `changes`. For example, here's a template that you
# can copy and paste just below the line `changes:`:
# ```
#  - stage: <STAGE NUMBER HERE>
#    start: <START TIME HERE>
#    finsh: <FINISH TIME HERE>
#    source: <URL TO INFORMATION SOURCE HERE>
#    exclude: <coct if this schedule doesn't apply to cape town>
#    include: <coct if this schedule only applies to cape town>
# ```
# See the README.md for more details
---
"""

bearer_token = os.environ["TWITTER_BEARER_TOKEN"]

COCT_USER_ID = 132437983
ESKOM_USER_ID = 466420346


def make_pr_body_text(source, tweet_dt, tweet_text, yaml_text):
    return f"""
Hey @beyarkay,

Cape Town [tweeted]({source}) at {tweet_dt}:

<table>
<thead>
<td>Tweet text
<td>Parsed YAML
<tr>
<td>

{tweet_text}

<td>

```yaml
{yaml_text}
```

</table>
"""


def check(response, status_code=200):
    """Checks the response for a 200 status code, or raises an exception."""
    if response.status_code != status_code:
        raise Exception(
            f"Request returned an error: {response.status_code} {response.text}"
        )
    return response


def get_tweets():
    print("Getting tweets for CPT")

    def bearer_oauth(r):
        """Method required by bearer token authentication."""
        r.headers["Authorization"] = f"Bearer {bearer_token}"
        r.headers["User-Agent"] = "eskom-calendar-auto-pr-bot"
        return r

    return check(
        requests.request(
            "GET",
            f"https://api.twitter.com/2/users/{COCT_USER_ID}/tweets",
            auth=bearer_oauth,
            params={"tweet.fields": "created_at"},
        )
    ).json()


def parse_into_loadshedding(tweet_json: dict):
    """Given some JSON describing a tweet, attempt to pull Loadshedding data
    from it. Returns an empty list on failure."""
    tweet = tweet_json["text"].replace("\n\n", "\n")
    tweet_id = tweet_json["id"]
    dt = datetime.datetime.strptime(tweet_json["created_at"], "%Y-%m-%dT%H:%M:%S.%fZ")

    change_re = r"Stage\s+(\d):\s+(\d\d:\d\d)\s+-\s+(\d\d:\d\d)"
    date_re = r"^\d\d? (jan(uary)?|feb(uary)?|mar(ch)?|apr(il)?|may|june?|july?|aug(ust)?|sep(tember)?|oct(ober)?|nov(ember)?|dec(ember)?)"  # noqa: E501

    changes = []
    curr_date = None
    source = f"https://twitter.com/CityofCT/status/{tweet_id}"

    # Try to parse each line in the tweet
    for line in tweet.splitlines():
        if match := re.match(date_re, line, flags=re.IGNORECASE):
            # First check if we've got a date
            # like `25 April ` or `26 May`
            curr_date = datetime.datetime.strptime(match.string, "%d %b")
            # We'll assume the year to be the same as that of the tweet. NYE
            # will be covered in the elif.
            curr_date = curr_date.replace(year=dt.year)
        elif match := re.match(change_re, line):
            # Now we check if the line is a LS change like:
            # Stage 0 (no load-shedding): 05:00 - 16:00
            # Stage 4: 20:00 - 05:00
            # Stage 6: 22:00 - 05:00
            stage, start, finsh = match.groups()
            start = datetime.datetime.strptime(start, "%H:%M").replace(
                year=curr_date.year,
                month=curr_date.month,
                day=curr_date.day,
            )
            finsh = datetime.datetime.strptime(finsh, "%H:%M").replace(
                year=curr_date.year,
                month=curr_date.month,
                day=curr_date.day,
            )
            # Cover the case when the loadshedding change goes over midnight.
            # (22:00 though to 05:00). This also covers the Month/year boundary
            # case
            if start > finsh:
                finsh = finsh + datetime.timedelta(days=1)
            # Convert the stage to an int
            stage = int(stage)
            # Convert the data to a dict and append it to the changes array
            changes.append(make_change(stage, start, finsh, source, None, "coct"))

    return changes


def make_change(
    stage: int,
    start: datetime.datetime,
    finsh: datetime.datetime,
    source: str,
    exclude: str | None,
    include: str | None,
):
    """Convert some fields into a nicely structured dict."""
    return (
        {
            "stage": int(stage),
            "start": start.isoformat(sep="T"),
            "finsh": finsh.isoformat(sep="T"),
            "source": source,
        }
        | ({} if exclude is None else {"exclude": exclude})
        | ({} if include is None else {"include": include})
    )


def make_new_branch(base_url, headers, branch_name):
    print(
        f"Making new branch https://github.com/beyarkay/eskom-calendar/tree/{branch_name}"  # noqa: E501
    )
    get_head_ref_request = check(
        requests.get(f"{base_url}/git/ref/heads/main", headers=headers)
    )
    ref = get_head_ref_request.json()["object"]["sha"]

    # TODO this doesn't do anything if the reference already exists
    # Create a new branch
    data = {"ref": f"refs/heads/{branch_name}", "sha": ref}
    _response = check(
        requests.post(f"{base_url}/git/refs", headers=headers, json=data),
        status_code=201,
    )
    print(f"New branch `{branch_name}` created successfully!")


def write_content_to_manually_specified(new_changes, base_url, headers, branch_name):
    print(
        f"Writing {len(new_changes)} new changes to https://github.com/beyarkay/eskom-calendar/tree/{branch_name}"  # noqa: E501
    )
    get_man_spec_request = check(
        requests.get(
            f"{base_url}/contents/manually_specified.yaml",
            headers=headers,
        )
    )
    file_data = get_man_spec_request.json()

    # Convert the base64 file content to a YAML string
    orig_content = (base64.b64decode(file_data["content"])).decode("utf-8")
    # Convert the YAML string to a dict
    content = yaml.safe_load(orig_content)
    # TODO don't just blindly replace, but rather:
    # 1. Remove all outdated entries and
    # 2. Only remove entries which conflict with the current entries Add all
    #    the new City of Cape Town entries

    # Remove all the old City of Cape Town entries in the `changes` item
    content["changes"]: list = [
        e for e in content["changes"] if e.get("include") != "coct"
    ]
    content["changes"].extend(new_changes)

    # Setup YAML to print datetimes properly
    def datetime_representer(dumper, value: datetime.datetime):
        return dumper.represent_scalar(
            "tag:yaml.org,2002:timestamp", value.isoformat(sep="T")
        )

    # Add the custom representer to the SafeDumper object
    yaml.SafeDumper.add_representer(datetime.datetime, datetime_representer)

    text = PRELUDE + yaml.safe_dump(
        data=content,
        stream=None,
        default_flow_style=False,
        sort_keys=False,
        indent=2,
    )
    print(text)

    # Convert base64-encoded bytes to string
    file_data["content"] = (base64.b64encode(text.encode("utf-8"))).decode("utf-8")
    file_data["message"] = "Update manually_specified with twitter data"
    file_data["branch"] = branch_name

    # Commit the changes
    get_man_spec_request = check(
        requests.put(
            f"{base_url}/contents/manually_specified.yaml",
            headers=headers,
            json=file_data,
        )
    )


def make_pull_request(headers, branch_name, source, tweet_dt, tweet_text, yaml_text):
    print(f"Making a pull request {branch_name}->main for tweet {source}")
    data = {
        "title": f"CPT Update: {source}",
        "body": make_pr_body_text(source, tweet_dt, tweet_text, yaml_text),
        "base": "main",
        "head": branch_name,
        "maintainer_can_modify": True,
    }

    _response = check(
        requests.post(
            "https://api.github.com/repos/beyarkay/eskom-calendar/pulls",
            headers=headers,
            data=json.dumps(data),
        ),
        status_code=201,
    )


def main():
    # 1. Get the latest tweets
    json_response = get_tweets()
    # 2. Get the first tweet which concerns load shedding, and convert it to a
    # list of changes
    selected_tweet = next(
        tweet_json
        for tweet_json in json_response["data"]
        if parse_into_loadshedding(tweet_json)
    )
    new_changes = parse_into_loadshedding(selected_tweet)
    print(
        f"Chosen tweet is "
        f"https://twitter.com/CityofCT/status/{selected_tweet['id']} "
        f"for conversion into loadshedding changes"
    )

    # 3. Setup some useful variables
    base_url = "https://api.github.com/repos/beyarkay/eskom-calendar"
    github_pat = os.environ["GITHUB_PAT"]
    headers = {
        "Authorization": f"token {github_pat}",
        "Accept": "application/vnd.github.v3+json",
    }

    # 4. Create a new branch
    branch_name = "auto-pr-bot"
    make_new_branch(base_url, headers, branch_name)

    # 5. Write the changes to the manually_specified file on that branch
    write_content_to_manually_specified(new_changes, base_url, headers, branch_name)

    # 6. Open a PR to merge that branch into main
    make_pull_request(
        headers,
        branch_name,
        new_changes[0]["source"],
        datetime.datetime.strptime(
            selected_tweet["created_at"], "%Y-%m-%dT%H:%M:%S.%fZ"
        ),
        selected_tweet["text"],
        yaml.safe_dump(
            data=new_changes, stream=None, default_flow_style=False, sort_keys=False
        ),
    )


if __name__ == "__main__":
    main()
