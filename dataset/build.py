# /// script
# requires-python = ">=3.11"
# dependencies = ["polars>=1.0", "pyyaml>=6.0", "tqdm>=4.66"]
# ///
"""Build the historical eskom-calendar dataset (July 2022 to March 2024) from git history.

    uv run dataset/build.py
    uv run dataset/build.py --areas 'city-of-cape-town-area-1$|city-power-1$'  # quick sample

Every version of `manually_specified.yaml` is replayed into one stage timeline, which is
then joined onto the area schedules (as they were at the time) by the same Rust code that
generated the published calendars. Output goes to `dataset/out/`, ready to upload to
Hugging Face. See `dataset/README.md` for the method and its caveats.
"""

import argparse
import bisect
import json
import re
import shutil
import subprocess
import tempfile
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from pathlib import Path

import polars as pl
import yaml
from tqdm import tqdm

REPO = Path(__file__).resolve().parent.parent
OUT = REPO / "dataset" / "out"
BINARY = REPO / "target" / "release" / "eskom-calendar"
SAST = timezone(timedelta(hours=2))
TZ = "Africa/Johannesburg"
AREA_CHUNK = 300
CHANGE_CHUNK = 300

# Last commit on main before the project went dormant (6 March 2024)
FINAL_COMMIT = "f9a3e54bd21e99eaf3d3937a4d331c4aa63236fb"

# Commits where City Power published a new schedule, as opposed to parser bug fixes, which
# are applied to the whole history
CITY_POWER_UPDATES = [
    "59ff68f802355c0ffc00262945eb41cda31c410b",  # 2023 rev 0
    "51a8051a64f913ddb4c0d55209df0112ff45dd0b",  # 2023-11 rev 4
    "5034b7065727e648a4160b084d4d1b41f438d2af",  # dated 25/12/23
]

# eThekwini replaced blocks 1-16 with a new set of blocks on this date (see src/main.rs)
ETHEKWINI_SWITCH = datetime(2023, 5, 25)
ETHEKWINI_OLD_BLOCKS = {f"kwazulu-natal-ethekwini-block-{i}" for i in range(1, 17)}

SCOPES = {"city_of_cape_town": {"include": "coct"}, "all_other_areas": {"exclude": "coct"}}


def git(*args: str) -> str:
    return subprocess.run(
        ["git", "-c", "core.quotepath=off", "-C", str(REPO), *args],
        capture_output=True,
        text=True,
        check=True,
    ).stdout


def sast(iso: str) -> datetime:
    return datetime.fromisoformat(iso).astimezone(SAST).replace(tzinfo=None)


@dataclass
class Commit:
    sha: str
    committed_at: datetime
    raw: list[tuple[str, int, dict]]
    entries: dict[str, list[tuple]] | None = None
    skip_reason: str | None = None


def parse_entry(c: dict) -> tuple[datetime, datetime, int, str, tuple[str, ...]]:
    start = datetime.fromisoformat(c["start"])
    end = datetime.fromisoformat(c["finsh"])
    # Early entries ended at hh:59 to dodge the overlap check; they mean the next full hour
    if end.minute == 59:
        end += timedelta(minutes=1)
    stage = int(c["stage"])
    if end <= start or not 0 <= stage <= 8:
        raise ValueError(f"impossible entry {c}")
    if c.get("include_regex") or c.get("exclude_regex"):
        raise ValueError(f"unsupported regex scope {c}")
    scope = (c.get("include"), c.get("exclude"))
    if scope == ("coct", None):
        scopes = ("city_of_cape_town",)
    elif scope == (None, "coct"):
        scopes = ("all_other_areas",)
    elif scope == (None, None):
        scopes = tuple(SCOPES)
    else:
        raise ValueError(f"unknown scope {c}")
    return start, end, stage, c.get("source", ""), scopes


def read_commits() -> list[Commit]:
    log = git("log", "--reverse", "--format=%H %cI", FINAL_COMMIT, "--", "manually_specified.yaml")
    commits = []
    for line in tqdm(log.splitlines(), desc="reading manually_specified.yaml history"):
        sha, committed = line.split()
        commit = Commit(sha, sast(committed), [])
        commits.append(commit)
        try:
            doc = yaml.load(git("show", f"{sha}:manually_specified.yaml"), Loader=yaml.BaseLoader)
        except yaml.YAMLError as e:
            commit.skip_reason = f"YAML does not parse: {str(e).splitlines()[0]}"
            continue
        for list_name in ("changes", "historical_changes"):
            for i, c in enumerate((doc or {}).get(list_name) or []):
                commit.raw.append((list_name, i, c))
        try:
            entries = [parse_entry(c) for _, _, c in commit.raw]
        except (KeyError, ValueError, TypeError) as e:
            commit.skip_reason = f"invalid entry: {e}"
            continue
        by_scope = {s: sorted(e[:4] for e in entries if s in e[4]) for s in SCOPES}
        if any(a[1] > b[0] for segs in by_scope.values() for a, b in zip(segs, segs[1:])):
            commit.skip_reason = "overlapping entries"
            continue
        commit.entries = by_scope
    return commits


def resolve(commits: list[Commit], rule: str) -> dict[str, list[tuple]]:
    """Replay the commits into one non-overlapping (start, end, stage, source, commit) timeline.

    rule="final": a commit overrides everything from its earliest entry onwards, so later
        corrections win, and entries that were pruned (always oldest-first) are kept.
    rule="realtime": a commit only overrides from its commit time onwards, which gives the
        schedule as it was known at the time.
    """
    timeline = {s: [] for s in SCOPES}
    first = True
    for commit in commits:
        if commit.entries is None:
            continue
        for s, entries in commit.entries.items():
            if first:
                cutoff = datetime.min
            elif rule == "final":
                cutoff = min([commit.committed_at] + [e[0] for e in entries])
            else:
                cutoff = commit.committed_at
            kept = [(a, min(b, cutoff), *rest) for a, b, *rest in timeline[s] if a < cutoff]
            new = [
                (max(a, cutoff), b, stage, source, commit.sha)
                for a, b, stage, source in entries
                if b > cutoff and stage > 0
            ]
            timeline[s] = sorted(kept + new)
        first = False

    for s, segs in timeline.items():
        merged = []
        for seg in segs:
            if merged and merged[-1][1] == seg[0] and merged[-1][2:4] == seg[2:4]:
                merged[-1] = (merged[-1][0], seg[1], *merged[-1][2:])
            else:
                merged.append(seg)
        timeline[s] = merged
    return timeline


def disagreement_hours(t1: list[tuple], t2: list[tuple]) -> float:
    def stage_at(segs, starts, t):
        i = bisect.bisect_right(starts, t) - 1
        return segs[i][2] if i >= 0 and segs[i][1] > t else 0

    s1, s2 = [s[0] for s in t1], [s[0] for s in t2]
    bounds = sorted({x for seg in t1 + t2 for x in seg[:2]})
    total = timedelta()
    for a, b in zip(bounds, bounds[1:]):
        if stage_at(t1, s1, a) != stage_at(t2, s2, a):
            total += b - a
    return total.total_seconds() / 3600


@dataclass
class ScheduleVersion:
    area: str
    csv_text: str
    commit: str
    valid_from: datetime | None
    valid_to: datetime | None


def first_added() -> dict[str, datetime]:
    log = git(
        "log",
        "--reverse",
        "-M",
        "--diff-filter=AR",
        "--format=>%cI",
        "--name-status",
        FINAL_COMMIT,
        "--",
        "generated/",
    )
    added = {}
    for line in log.splitlines():
        if line.startswith(">"):
            when = sast(line[1:])
        elif line.endswith(".csv"):
            status, *paths = line.split("\t")
            old, new = Path(paths[0]).stem, Path(paths[-1]).stem
            # A renamed file keeps the date it was first added under its old name
            added.setdefault(new, added.get(old, when) if status.startswith("R") else when)
    return added


def schedule_versions() -> list[ScheduleVersion]:
    tree = git("ls-tree", "--name-only", FINAL_COMMIT, "generated/").split()
    paths = [p for p in tree if p.endswith(".csv")]
    versions = []
    for path in tqdm(paths, desc="reading area schedules"):
        area = Path(path).stem
        if area.startswith("city-power-"):
            continue
        text = git("show", f"{FINAL_COMMIT}:{path}")
        lo, hi = None, None
        if area.startswith("kwazulu-natal-ethekwini-"):
            lo, hi = (None, ETHEKWINI_SWITCH) if area in ETHEKWINI_OLD_BLOCKS else (ETHEKWINI_SWITCH, None)
        versions.append(ScheduleVersion(area, text, FINAL_COMMIT, lo, hi))

    switch_times = [sast(git("show", "-s", "--format=%cI", sha).strip()) for sha in CITY_POWER_UPDATES]
    sources = [f"{CITY_POWER_UPDATES[0]}^", *CITY_POWER_UPDATES[:-1], FINAL_COMMIT]
    bounds = [None, *switch_times, None]
    for i, rev in enumerate(sources):
        sha = git("rev-parse", rev).strip()
        for path in git("ls-tree", "--name-only", sha, "generated/").split():
            if Path(path).name.startswith("city-power-") and path.endswith(".csv"):
                text = git("show", f"{sha}:{path}")
                versions.append(ScheduleVersion(Path(path).stem, text, sha, bounds[i], bounds[i + 1]))
    return versions


def run_calendar_code(group: list[ScheduleVersion], manually_specified: str, added: dict) -> pl.DataFrame:
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        (tmp / "generated").mkdir()
        (tmp / "calendars").mkdir()
        for v in group:
            (tmp / "generated" / f"{v.area}.csv").write_text(v.csv_text)
        (tmp / "manually_specified.yaml").write_text(manually_specified)
        result = subprocess.run([BINARY, "--output-ics-files=false"], cwd=tmp, capture_output=True, text=True)
        if result.returncode != 0:
            raise RuntimeError(
                f"eskom-calendar failed for {[v.area for v in group]}:\n{result.stderr[-3000:]}"
            )
        df = pl.read_csv(tmp / "calendars" / "machine_friendly.csv", schema_overrides={"source": pl.String})
    versions = pl.DataFrame(
        {
            "area_name": [v.area for v in group],
            "schedule_commit": [v.commit for v in group],
            "first_era": [v.valid_from is None for v in group],
            "area_added_at": [added[v.area] for v in group],
        }
    )
    return df.join(versions, on="area_name", how="left")


def join(versions: list[ScheduleVersion], timeline: dict[str, list[tuple]], added: dict) -> pl.DataFrame:
    groups = defaultdict(list)
    for v in versions:
        groups[(v.valid_from, v.valid_to)].append(v)

    jobs = []
    for (lo, hi), group in groups.items():
        changes = []
        for scope, segs in timeline.items():
            for a, b, stage, source, _ in segs:
                a, b = max(a, lo or a), min(b, hi or b)
                if a < b:
                    changes.append(
                        {"stage": stage, "start": a.isoformat(), "finsh": b.isoformat(), "source": source}
                        | SCOPES[scope]
                    )
        # Each change's outages are independent of the others, so the calendar code can be run
        # on slices of the changes; its run time grows faster than linearly with their number
        for j in range(0, len(changes), CHANGE_CHUNK):
            manually_specified = yaml.safe_dump(
                {"changes": changes[j : j + CHANGE_CHUNK], "historical_changes": []}, sort_keys=False
            )
            for i in range(0, len(group), AREA_CHUNK):
                jobs.append((group[i : i + AREA_CHUNK], manually_specified))

    # One run at a time: each run of the calendar code peaks at about 4 GB regardless of its size
    shards = OUT / "shards"
    shards.mkdir(exist_ok=True)
    for i, (group, manually_specified) in enumerate(tqdm(jobs, desc="joining stages onto area schedules")):
        run_calendar_code(group, manually_specified, added).write_parquet(shards / f"{i:04}.parquet")
    df = pl.read_parquet(shards / "*.parquet")
    shutil.rmtree(shards)
    ts = lambda c: pl.col(c).str.to_datetime("%Y-%m-%dT%H:%M:%S%z").dt.convert_time_zone(TZ)
    added_tz = pl.col("area_added_at").dt.replace_time_zone(TZ)
    return (
        df.with_columns(start=ts("start"), end=ts("finsh"), stage=pl.col("stage").cast(pl.Int8))
        .with_columns(schedule_backfilled=pl.col("first_era") & (pl.col("start") < added_tz))
        .select("area_name", "start", "end", "stage", "source", "schedule_commit", "schedule_backfilled")
        .sort("area_name", "start", "stage")
    )


def timeline_frame(timeline: dict[str, list[tuple]]) -> pl.DataFrame:
    rows = [
        {"applies_to": s, "start": a, "end": b, "stage": stage, "source": source, "commit": sha}
        for s, segs in timeline.items()
        for a, b, stage, source, sha in segs
    ]
    return pl.DataFrame(rows).with_columns(
        pl.col("start", "end").dt.replace_time_zone(TZ), pl.col("stage").cast(pl.Int8)
    )


def history_frame(commits: list[Commit]) -> pl.DataFrame:
    rows = []
    for c in commits:
        base = {
            "commit": c.sha,
            "committed_at": c.committed_at,
            "commit_used": c.entries is not None,
            "commit_skip_reason": c.skip_reason,
        }
        empty = {
            "list": None,
            "position": None,
            "stage": None,
            "start": None,
            "end": None,
            "source": None,
            "include": None,
            "exclude": None,
        }
        if not c.raw:
            rows.append(base | empty)
        for list_name, i, e in c.raw:
            rows.append(
                base
                | {
                    "list": list_name,
                    "position": i,
                    "stage": e.get("stage"),
                    "start": e.get("start"),
                    "end": e.get("finsh"),
                    "source": e.get("source"),
                    "include": e.get("include"),
                    "exclude": e.get("exclude"),
                }
            )
    return pl.DataFrame(rows, infer_schema_length=None).with_columns(
        pl.col("committed_at").dt.replace_time_zone(TZ),
        pl.col("stage").cast(pl.Int8, strict=False),
        pl.col("position").cast(pl.Int32),
    )


def schedules_frame(versions: list[ScheduleVersion], added: dict) -> pl.DataFrame:
    day_columns = {"date_of_month": "monthly", "day_of_week": "weekly", "day_of_cycle": "periodic"}
    frames = []
    for v in versions:
        df = pl.read_csv(v.csv_text.encode(), comment_prefix="#", infer_schema_length=0)
        df = df.rename({c: c.strip() for c in df.columns})
        day_col = next(c for c in day_columns if c in df.columns)
        frames.append(
            df.select(
                pl.lit(v.area).alias("area_name"),
                pl.lit(day_columns[day_col]).alias("recurrence"),
                pl.col(day_col).str.strip_chars().cast(pl.Int16).alias("day"),
                (
                    pl.col("period_of_cycle").str.strip_chars().cast(pl.Int16)
                    if "period_of_cycle" in df.columns
                    else pl.lit(None, pl.Int16)
                ).alias("period_of_cycle"),
                (
                    pl.col("start_of_cycle").str.strip_chars()
                    if "start_of_cycle" in df.columns
                    else pl.lit(None, pl.String)
                ).alias("start_of_cycle"),
                pl.col("start_time").str.strip_chars().str.slice(0, 5).alias("start_time"),
                pl.col("finsh_time").str.strip_chars().str.slice(0, 5).alias("end_time"),
                pl.col("stage").str.strip_chars().cast(pl.Int8),
                pl.lit(v.valid_from, pl.Datetime("us")).alias("valid_from"),
                pl.lit(v.valid_to, pl.Datetime("us")).alias("valid_to"),
                pl.lit(v.commit).alias("schedule_commit"),
                pl.lit(added[v.area], pl.Datetime("us")).alias("area_added_at"),
            )
        )
    return (
        pl.concat(frames)
        .with_columns(pl.col("valid_from", "valid_to", "area_added_at").dt.replace_time_zone(TZ))
        .filter(pl.col("stage") > 0)
        .sort("area_name", "valid_from", "recurrence", "day", "start_time", "stage", nulls_last=False)
    )


def area_frames(added: dict) -> tuple[pl.DataFrame, pl.DataFrame]:
    meta = yaml.safe_load(git("show", f"{FINAL_COMMIT}:area_metadata.yaml"))["area_details"]
    areas, suburbs = {}, []
    for entry in meta:
        name = entry["calendar_name"].removesuffix(".ics")
        areas.setdefault(
            name,
            {
                "area_name": name,
                "provider": entry.get("provider"),
                "province": entry.get("province"),
                "municipality": entry.get("municipality"),
                "city": entry.get("city"),
                "schedule_source": entry.get("source"),
                "schedule_source_info": entry.get("source_info"),
                "area_added_at": added.get(name),
            },
        )
        for a in entry.get("areas") or []:
            names = a["name"] if isinstance(a["name"], list) else [a["name"]]
            for suburb in names:
                suburbs.append(
                    {
                        "area_name": name,
                        "suburb": str(suburb),
                        "province": a.get("province", entry.get("province")),
                        "municipality": a.get("municipality", entry.get("municipality")),
                    }
                )
    areas_df = (
        pl.DataFrame(list(areas.values()))
        .with_columns(pl.col("area_added_at").dt.replace_time_zone(TZ))
        .sort("area_name")
    )
    suburbs_df = pl.DataFrame(suburbs).unique(maintain_order=True).sort("area_name", "suburb")
    return areas_df, suburbs_df


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        "--areas", help="only build outages for areas matching this regex, to check a change quickly"
    )
    args = parser.parse_args()

    subprocess.run(["cargo", "build", "--release"], cwd=REPO, check=True)
    shutil.rmtree(OUT, ignore_errors=True)
    (OUT / "data").mkdir(parents=True)

    commits = read_commits()
    final = resolve(commits, "final")
    realtime = resolve(commits, "realtime")
    added = first_added()
    versions = schedule_versions()
    if args.areas:
        versions = [v for v in versions if re.search(args.areas, v.area)]

    areas, suburbs = area_frames(added)
    tables = {
        "outages": join(versions, final, added),
        "stage_timeline": timeline_frame(final),
        "areas": areas,
        "suburbs": suburbs,
        "area_schedules": schedules_frame(versions, added),
        "stage_file_history": history_frame(commits),
    }
    for name, df in tables.items():
        df.write_parquet(OUT / "data" / f"{name}.parquet", compression="zstd", statistics=True)
    shutil.copy(REPO / "dataset" / "README.md", OUT / "README.md")

    timeline = tables["stage_timeline"].with_columns(
        hours=(pl.col("end") - pl.col("start")).dt.total_minutes() / 60
    )
    outages = tables["outages"]
    summary = {
        "rows": {name: df.height for name, df in tables.items()},
        "commits": len(commits),
        "commits_skipped": [
            (c.committed_at.isoformat(), c.skip_reason) for c in commits if c.entries is None
        ],
        "timeline_range": [str(timeline["start"].min()), str(timeline["end"].max())],
        "stage_hours_by_year": timeline.group_by("applies_to", pl.col("start").dt.year().alias("year"))
        .agg(pl.col("hours").sum().round(1))
        .sort("applies_to", "year")
        .to_dicts(),
        "final_vs_realtime_disagreement_hours": {
            s: round(disagreement_hours(final[s], realtime[s]), 1) for s in SCOPES
        },
        "areas": outages["area_name"].n_unique(),
        "backfilled_rows": outages["schedule_backfilled"].sum(),
        "outage_range": [str(outages["start"].min()), str(outages["end"].max())],
    }
    (OUT / "summary.json").write_text(json.dumps(summary, indent=2, default=str))
    print(json.dumps(summary, indent=2, default=str))


if __name__ == "__main__":
    main()
