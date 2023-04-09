# Welcome to the eskom-calendar contributing guide!

First off, thanks for your interest in the project! We really appreciate it.
There are lots of ways to help out, and not all of them require knowledge about
programming.

> If you like the project but just don't have time to contribute, that's fine!
> There are other easy ways to support the project and show your appreciation
> which we would also be very happy about:
> - Star the project
> - Share the website with your friends/family (https://eskomcalendar.co.za)
> - Use the open-source loadshedding schedules in your own projects, and let us
>   know about it!

If you don't know how eskom-calendar works at a high level, read [this](TODO)
section of the README first.

## How to help

At a high level, there are 3 ways to help out. The easiest way is to update
`manually_specified.yaml` with new loadshedding schedules when Eskom or the
city of Cape Town announces them. The second way is to file bug reports or
to find areas of South Africa which don't have their loadshedding schedules
listed on [eskomcalendar.co.za](https://eskomcalendar.co.za). The third way is
through code contributions, which could be adding new features or fixing bugs.
I'll discuss each of these three options in greater detail below:

### 1. Keeping eskom-calendar up-to-date with loadshedding schedules

In South Africa, loadshedding is controlled by Eskom, who will announce
loadshedding schedules via their twitter account. Each tweet they announce only
covers about 3 days worth of loadshedding before they announce a new schedule,
so more or less every three days, someone has to manually convert their tweet into
something machine-readable.

This is done by opening a Pull Request (PR) which edits the file
`manually_specified.yaml`. When a maintainer approves your edits, then a load
of automated processes will start which update the calendars and the
loadshedding schedules, and send those updates off to whoever has subscribed to
eskom-calendar.

If you don't know [YAML](https://quickref.me/yaml), don't worry, it's super
simple (and there's a bot that ensures everything is valid when you open a PR).
`manually_specified.yaml` has a list of `Change` objects, and each `Change`
object defines one 'item' of loadshedding (ie Stage 2 from 05:00 to 22:00 in
Cape Town). It looks a bit like this:

```yaml
changes:
    # Load shedding for Eskom
  - stage: 3
    start: 2023-04-09T17:00:00 # Note: No timezone
    finsh: 2023-04-10T05:00:00 # Note: `finsh`, and not `finish`
    source: https://twitter.com/Eskom_SA/status/1645044545434918914
    exclude: coct # Don't include Cape Town for this change

  # Load shedding for the City of Cape Town (CoCT)
  - stage: 2
    start: 2023-04-09T17:00:00
    finsh: 2023-04-10T05:00:00
    source: https://twitter.com/CityofCT/status/1645066898109767683
    include: coct # Only include Cape Town for this change
```

If that makes sense to you, then feel free to make your edits (click
[here](https://github.com/beyarkay/eskom-calendar/edit/main/manually_specified.yaml)).
We've got a bot which double checks everything is okay when you open a Pull
Request, so there's no danger of you breaking anything.

You can also use [this eskom-calendar
website](https://eskomcalendar.co.za/ec/pr) as an interactive, mobile-friendly
tool to help format the YAML for you, although it isn't super user-friendly at
the moment.

If you like knowing all the rules and rough corners, here are some things to note:

- The finish time is spelt without the second `i`, so that it lines up with
  `start`: `finsh`.
- The `start` and `finsh` datetimes are assumed to be in the South African
  Timezone (UTC+02:00), but otherwise are written according to RFC3339.
- The `source` is required, and can be any URL. This will almost always be
  a link to a specific tweet by a government body such as Eskom.
    - Because there can be a lot of hearsay about whether or not the
      loadshedding schedule will be updated, only first-party sources are
      allowed (you can't link a news article saying "An unnamed Eskom
      spokesperson told us that they'd go to Stage 8 on Sunday".
- Because some municipalities try to give separate loadshedding schedules to
  their customers (ie Cape Town), you can specify which areas your any one
  change is for.

  For example, if you specify `include: coct`, then that loadshedding change
  will *only* affect the city of cape town areas. If you specify `exclude:
  coct`, then that change will affect all areas except those in the city of
  cape town.
- There is bot that runs a YAML format check whenever `manually_specified.yaml`
  is updated, so make sure there aren't any trailing spaces, tabs, and no more
  than 2 consecutive empty lines. If you're not sure, you can always just open
  the PR and the bot will check it for you.
- There is a bot that runs a loadshedding logic check, to make sure that you
  didn't accidentally specify an invalid schedule (like scheduling stage 3 and
  stage 1 for the same areas, at the same times, on the same dates).

### 2. Missing area schedules and bug reports

Our beautiful country is full of different towns and cities, so figuring out if
eskom-calendar actually has loadshedding schedules for *all* of them is a hard
problem. If you share this project with your friends and ask them to check that
their home town is included, that's amazing!

If you (or your friends) find somewhere that's not mentioned, please [open an
issue](https://github.com/beyarkay/eskom-calendar/issues/new?assignees=beyarkay&labels=missing-area-schedule%2C+waiting-on-maintainer&template=missing-loadshedding-area-suburb.md&title=Missing+area+schedule).
It's the best way to ensure all of South Africa has coverage.

If you find a bug, please [click
here](https://github.com/beyarkay/eskom-calendar/issues/new)! It's really
important to us that this is something you can rely on, so bug-free code is
a must.

### 3. Code improvements, bug fixes, and performance improvements

Hello, world! Good to see a fellow coder taking a perusal though this
repository. We've tried to make the project as friendly as possible to new
developers, but please ask questions through GitHub issues if you've got any or are just curious.



#### Project structure

`eskom-calendar` is a [Rust](https://www.rust-lang.org/) project that uses (and
abuses) GitHub's free services. This allows the project have zero running
costs.

GitHub actions are used to convert the loadshedding schedules into ICS calendar
files for humans to enjoy and CSV files for machines to read. These files are
then uploaded to [a GitHub
Release](https://github.com/beyarkay/eskom-calendar/releases/tag/latest) which
provides an ever-constant URL for each calendar file. This means that users can
copy the link to that file, paste it into their calendar app, and then never
have to worry about loadshedding updates again.

In the long term we will move away from GitHub hosting, because it does not provide the level of control we'd ideally want, but it is a fantastic solution for now.

#### How it works

At a fairly low level, here's how the process of updating the schedule works:

1. Eskom, the City of Cape Town, or another municipality will announce an
   update to the loadshedding schedules
1. Someone will edit `manually_specified.yaml`, and commit that edit to the
   `main` branch.
1. The
   [`publish-calendars`](https://github.com/beyarkay/eskom-calendar/blob/main/.github/workflows/publish-calendars.yaml)
   github action will kick off automatically.
1. This action will check that the edits are valid,
1. It will then start to build the calendars using `src/main.rs`.
1. Once the calendars are built, they are uploaded to the GitHub release, at
   which point they are available to the world.

There is also the website, [eskomcalendar.co.za](https://eskomcalendar.co.za/)
which is powered by GitHub pages and hosted on Boyd's personal GitHub account
(see the repo at
[beyarkay/beyarkay.github.io](https://github.com/beyarkay/beyarkay.github.io).
It is a React app which allows users to search through the calendars and view
the one they want. It's not super complex, and could definitely do with some TLC.


#### Making code contributions

If you'd like to make changes, feel free to open an issue so we can discuss it
first. I have GitHub notifications setup and usually respond in a few hours.
I'll be able to guide you towards the right area of the code base.

If you're keen to do _something_ but don't know what, check out the
[`documentation`](https://github.com/beyarkay/eskom-calendar/issues?q=is%3Aopen+label%3Adocumentation+sort%3Aupdated-desc),
[`good-first-issue`](https://github.com/beyarkay/eskom-calendar/issues?q=is%3Aopen+label%3A%22good+first+issue%22+sort%3Aupdated-desc),
or the
[help-wanted](https://github.com/beyarkay/eskom-calendar/issues?q=is%3Aopen+label%3A%22help+wanted%22+sort%3Aupdated-desc)
tags. These are kept up-to-date with interesting ideas or extensions, and
usually have fleshed out descriptions. Just make sure you leave a comment if
you start working on something, so we know not to also try fix it ourselves.
