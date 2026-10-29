---
license: cc-by-nc-sa-4.0
language:
- en
pretty_name: "Eskom Calendar: scheduled loadshedding by area in South Africa, 2022-2024"
tags:
- load-shedding
- loadshedding
- south-africa
- eskom
- electricity
- power-outages
- energy
- time-series
size_categories:
- 1M<n<10M
configs:
- config_name: outages
  default: true
  data_files: data/outages.parquet
- config_name: stage_timeline
  data_files: data/stage_timeline.parquet
- config_name: areas
  data_files: data/areas.parquet
- config_name: suburbs
  data_files: data/suburbs.parquet
- config_name: area_schedules
  data_files: data/area_schedules.parquet
- config_name: stage_file_history
  data_files: data/stage_file_history.parquet
---

# Eskom Calendar: scheduled loadshedding by area in South Africa, 2022-2024

This dataset lists every scheduled loadshedding power cut for __AREAS__ South African
loadshedding areas between __START__ and __END__. Each row of the main table gives an area,
the start and end of a power cut, the loadshedding stage, and a link to the announcement
that set that stage.

It was rebuilt from the git history of
[eskom-calendar](https://github.com/beyarkay/eskom-calendar), an open-source project that
published loadshedding schedules as calendar files and a CSV between July 2022 and March
2024. The published CSV only ever held upcoming power cuts, and each update replaced the
previous one, so no single file ever held the history. The history survives because every
change to the announced stages was a git commit. This dataset replays all __COMMITS__
of those commits.

## Quick start

```python
import pandas as pd

base = "hf://datasets/__REPO_ID__/data"
outages = pd.read_parquet(f"{base}/outages.parquet")

# Hours each Cape Town area was scheduled to be without power in October 2023
cpt = outages[outages.area_name.str.startswith("city-of-cape-town-area-")]
oct23 = cpt[(cpt.start >= "2023-10-01") & (cpt.start < "2023-11-01")]
```

With the `datasets` library, `load_dataset("__REPO_ID__", "outages")` works, and each
table below is a separate config. Any table converts to CSV with
`pd.read_parquet(...).to_csv("outages.csv", index=False)`.

All timestamps are South African Standard Time (UTC+02:00, `Africa/Johannesburg`), which
has no daylight saving.

## What this data is, and what it is not

**These are scheduled power cuts, not observed ones.** A row means that, given the stage
announced at the time, the area's published schedule called for the power to be off. No
one checked whether it went off. Municipalities sometimes ran late, skipped slots, or had
unrelated faults, and some customers were exempt.

**Coverage is 13 July 2022 to 9 March 2024.** The project did not exist before July 2022
and stopped updating in March 2024. Loadshedding before then, which goes back to 2007, and
any after March 2024 are missing.

**Only the City of Cape Town has its own stages.** Cape Town used its own Steenbras
hydro-electric scheme to run a lower stage than the rest of the country much of the time. From 23 September
2022 the project tracked Cape Town's announced stages separately. Before that date, Cape
Town areas use the national stage, which will overstate Cape Town's power cuts when the
City ran a lower stage. Every other municipality, including the other metros (City Power,
Tshwane, Ekurhuleni, eThekwini, Nelson Mandela Bay and others), uses the national stage
announced by Eskom.

**Some area schedules are backfilled.** The project added municipalities over time. An
area's schedule is applied to dates before it was added to the project, and those rows have
`schedule_backfilled = true`. If you only want data the project itself would have
published, drop them. See [Area schedules](#2-area-schedules) for the exceptions.

**Rows overlap and split.** Eskom's schedules use 2.5-hour slots that start every two hours,
so one area's consecutive slots overlap by 30 minutes. A slot that spans a stage change is
split into two rows, one per stage. To get the time an area spent without power, merge each
area's intervals before summing, rather than adding up row durations.

**Short slots are kept.** The calendars hid power cuts of 30 minutes or less, which
municipalities usually did not carry out. This dataset keeps them, so filter on duration if
you want to match the calendars.

## Tables

### `outages` (default)

One row per scheduled power cut per area. __OUTAGE_ROWS__ rows.

| column | type | description |
|---|---|---|
| `area_name` | string | Loadshedding area, for example `city-of-cape-town-area-15`. Join to `areas` and `suburbs`. |
| `start` | timestamp | When the power was scheduled to go off. |
| `end` | timestamp | When the power was scheduled to come back. |
| `stage` | int8 | Loadshedding stage, 1 to 8. |
| `source` | string | Link to the announcement (usually a tweet from @Eskom_SA or @CityofCT) that set the stage. |
| `schedule_commit` | string | Commit of the eskom-calendar repository the area's schedule was read from. |
| `schedule_backfilled` | bool | True if this row predates the area being added to the project. |

### `stage_timeline`

The resolved stage at every moment, separately for Cape Town and for everywhere else. Gaps
mean no loadshedding. __TIMELINE_ROWS__ rows.

| column | type | description |
|---|---|---|
| `applies_to` | string | `city_of_cape_town` or `all_other_areas`. |
| `start`, `end` | timestamp | The period this stage applied. |
| `stage` | int8 | Loadshedding stage, 1 to 8. |
| `source` | string | Link to the announcement. |
| `commit` | string | The commit whose version of this period was used. |

### `areas`

One row per loadshedding area. __AREA_ROWS__ rows.

| column | type | description |
|---|---|---|
| `area_name` | string | Loadshedding area. |
| `provider` | string | Who supplies the area and publishes its schedule: `eskom` for areas Eskom supplies directly, otherwise the municipality (`city-of-cape-town`, `city-power`, `ethekwini`, and so on). |
| `province`, `municipality`, `city` | string | Where the area is, when the area lies in one place. Eskom's direct-supply areas can span several municipalities; see `suburbs`. |
| `schedule_source` | string | Link to the document the schedule was parsed from. Many of these links are now dead. |
| `schedule_source_info` | string | Link to the page that listed that document. |
| `area_added_at` | timestamp | When the area's schedule was first added to the project. |

### `suburbs`

Suburbs, towns and farms each area covers, as written in the source schedules. Spelling is
inconsistent and some names repeat, so expect to fuzzy-match. There are no boundaries or
coordinates. __SUBURB_ROWS__ rows.

| column | type | description |
|---|---|---|
| `area_name` | string | Loadshedding area. |
| `suburb` | string | Place name. |
| `province`, `municipality` | string | Where the place is, when known. |

### `area_schedules`

The recurring schedule behind each area: for each day and stage, the time slots when the
power is off. __SCHEDULE_ROWS__ rows.

| column | type | description |
|---|---|---|
| `area_name` | string | Loadshedding area. |
| `recurrence` | string | `monthly` (most areas: `day` is the day of the month), `weekly` (`day` 1 is Monday) or `periodic` (a cycle of `period_of_cycle` days starting on `start_of_cycle`). |
| `day` | int16 | Day of the month, week or cycle. |
| `period_of_cycle`, `start_of_cycle` | int16, string | Only for periodic schedules. |
| `start_time`, `end_time` | string | Slot start and end, `HH:MM`. A slot that ends before it starts runs past midnight. |
| `stage` | int8 | The slot applies at this stage. Most schedules list a slot at every stage it applies to. |
| `valid_from`, `valid_to` | timestamp | When this version of the schedule applied. Null means unbounded. |
| `schedule_commit` | string | Commit the schedule file was read from. |
| `area_added_at` | timestamp | When the area was first added to the project. |

### `stage_file_history`

The raw material: every entry of every version of the project's
[`manually_specified.yaml`](https://github.com/beyarkay/eskom-calendar/blob/main/manually_specified.yaml),
the hand-maintained file of announced stages. Use it to rebuild the timeline with different
rules. __HISTORY_ROWS__ rows.

| column | type | description |
|---|---|---|
| `commit`, `committed_at` | string, timestamp | The commit and when it was made. |
| `commit_used` | bool | False if the version was invalid and skipped (see `commit_skip_reason`). |
| `commit_skip_reason` | string | Why it was skipped. |
| `list` | string | `changes` or `historical_changes`. Null for versions with no entries. |
| `position` | int32 | Position of the entry in that list. |
| `stage` | int8 | Announced stage. |
| `start`, `end` | string | Exactly as written in the file, SAST without an offset. Not all are valid dates. |
| `source` | string | Link to the announcement. |
| `include`, `exclude` | string | `coct` limits an entry to (`include`) or away from (`exclude`) Cape Town. Both null means everywhere. |

## How it was built

The build script is
[`dataset/build.py`](https://github.com/beyarkay/eskom-calendar/blob/main/dataset/build.py).
It runs in three steps.

### 1. Stage timeline

Volunteers kept `manually_specified.yaml` up to date by hand, adding a stage, a start and
end time, and a source link each time Eskom or the City of Cape Town announced a change.
Entries were revised constantly: __REVISED__ entries were edited or deleted before they
ended, mostly because Eskom changed its plans. Finished entries were also pruned to keep the
file short.

The build replays every commit in order. Each commit overrides the timeline from its
earliest entry onward, so a later correction replaces whatever came before it, and pruned
entries, which were removed oldest first in __PRUNED_OLDEST_FIRST__ of __PRUNED_TOTAL__
cases, are kept. __SKIPPED__ commits could not have built the calendars (the YAML did not
parse, an entry had an impossible date, or entries overlapped) and are skipped, as they were
at the time. Early entries ended at `hh:59` to avoid tripping an overlap check; these are
read as ending on the hour.

A stricter rule, under which each commit only counts from the moment it was made, gives the
schedule as it was known in real time. It differs from the published timeline for
__DISAGREE_COCT__ hours in Cape Town and __DISAGREE_REST__ hours elsewhere, out of
__HOURS_COCT__ and __HOURS_REST__ hours of loadshedding. Most of the difference is the
delay between an announcement and the commit recording it.

### 2. Area schedules

Each area has a recurring schedule parsed from a municipality's or Eskom's published PDF or
spreadsheet. Fixes to the parsers apply to the whole history, because they correct the
reading of a schedule rather than change it. Two municipalities replaced their schedules
during the period, and these use the version in force at the time:

- **City Power** (Johannesburg) published new schedules three times. The commits that added
  them, on 8 November 2023, 22 November 2023 and 15 January 2024 (a schedule dated 25
  December 2023), mark the boundaries. Blocks 17 and 18 exist only in the last schedule.
- **eThekwini** (Durban) replaced blocks 1 to 16 with a new set of blocks on 25 May 2023.

### 3. Join

The stage timeline and the area schedules are joined by the same Rust code that generated
the published calendars, so each row matches what the calendars showed, or would have shown.

## Licence and attribution

The data is released under
[CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/): you may share and
adapt it for non-commercial purposes, if you credit the source and release derived data
under the same licence. Please credit it with a direct link to
[eskomcalendar.co.za](https://eskomcalendar.co.za) or
[github.com/beyarkay/eskom-calendar](https://github.com/beyarkay/eskom-calendar). For
commercial use, get in touch.

```bibtex
@misc{kane_eskom_calendar_2026,
  author       = {Kane, Boyd and {eskom-calendar contributors}},
  title        = {Eskom Calendar: scheduled loadshedding by area in South Africa, 2022--2024},
  year         = {2026},
  publisher    = {Hugging Face},
  howpublished = {\url{https://huggingface.co/datasets/__REPO_ID__}}
}
```

The stage announcements were collected by the project's contributors, listed in the
[repository README](https://github.com/beyarkay/eskom-calendar#contributors). The schedules
belong to Eskom and the municipalities that published them.

## Rebuilding

```sh
git clone https://github.com/beyarkay/eskom-calendar
cd eskom-calendar
uv run dataset/build.py  # needs git, uv and a Rust toolchain; writes dataset/out/
```

The build reads only git history up to commit `f9a3e54` (6 March 2024), so it gives the
same output on any later checkout.
