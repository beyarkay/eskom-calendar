<p align="center">
  <a href="https://github.com/beyarkay/eskom-calendar/releases/tag/latest">
    <img src="imgs/header.png" alt="eskom-calendar: loadshedding in your calendar">
  </a><br>
  <img src="https://img.shields.io/github/license/beyarkay/eskom-calendar" alt="license"><br>
  Loadshedding schedules in your digital calendar. No ads, up-to-date,
  and developer friendly.
</p>

<p align="center">
  <a href="https://github.com/beyarkay/eskom-calendar/releases/tag/latest">Get it</a> •
  <a href="#easy-to-understand-and-plan-around">Key Features</a> •
  <a href="#using-the-data-in-your-own-projects">How to use the data</a> •
  <a href="#project-goals-and-alternatives">Project goals & alternatives</a><br>
</p>

## How to Get It

[Click here](https://github.com/beyarkay/eskom-calendar/releases/tag/latest) and
follow the instructions to get it in your calendar. Keep reading to learn why eskom-calendar is
[very cool](https://twitter.com/hermux/status/1549851414771503110),
[the OG](https://twitter.com/AngusRedBlue/status/1574337420048351233), and
[amazing](https://www.linkedin.com/feed/update/urn:li:activity:6978329466259271680?commentUrn=urn%3Ali%3Acomment%3A%28activity%3A6978329466259271680%2C6978364430430388224%29).

## Key Features

### Easy to understand and plan around

eskom-calendar makes planning around loadshedding as easy as it gets. Subscribe
to the digital calendar for your area, and you'll see loadshedding in your
schedule on your phone, laptop, smartwatch, smartfridge, alles. We'll show you
loadshedding as far into the furture as Eskom allows us. 

### An event in your calendar means your lights are off

Many loadshedding apps don't actually tell you when your lights are off, or if
they do, it's difficult to find or only shows you the very next power outage.
eskom-calendar shows you all the times your lights will be off, right in your
digital calendar.

### Perfect for teams and businesses

If you're a team manager, add the calendars for your team members and know
exactly when everyone will go dark so you don't have someone dropping off in
the middle of a meeting.

Businesses can see loadshedding schedules for all their branches in one view,
and prepare accordingly. 

IT departments can automate turning on generators or shutting down servers (see
[Using the Data in Your Own Projects](#using-the-data-in-your-own-projects)).

### No adverts

eskom-calendar does one thing, and does it well. You get an event in your
calendar if your power is going to go off, and that's it. There's no adverts,
there's no bloat. Just loadshedding information. Doesn't get simpler than that,
does it?

### The *only* open source, automation friendly option

eskom-calendar was created by [Boyd Kane](https://github.com/beyarkay) because
there was no way for a casual coder to just get loadshedding information
programmatically without messing with API keys and whatnot.

To the best of our knowledge, this is the easiest way to automate away the
pain of loadshedding, and it's the only open-source option to provide the times
when power will be off, as opposed to just the loadshedding schedule for any
given area (please get in contact if I'm wrong!, would be great to collab).

## Using the Data in Your Own Projects

We are really interested to see what the developers of South Africa do with
this data source.

The main file of interest will be
[`machine_friendly.csv`](https://github.com/beyarkay/eskom-calendar/releases/download/latest/machine_friendly.csv)
built from the same source of information as the calendar files. It looks
something like:

```
       │ File: machine_friendly.csv
───────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
   1   │ area_name,start,finsh,stage,source
   2   │ kwazulu-natal-mpofana,2022-09-25T23:00:00+02:00,2022-09-26T01:30:00+02:00,3,"https://twitter.com/Eskom_SA/status/1574014612097454080"
   3   │ kwazulu-natal-mpofana,2022-09-26T07:00:00+02:00,2022-09-26T09:30:00+02:00,3,"https://twitter.com/Eskom_SA/status/1574014612097454080"
   4   │ kwazulu-natal-mpofana,2022-09-26T16:00:00+02:00,2022-09-26T17:30:00+02:00,4,"https://twitter.com/Eskom_SA/status/1574014612097454080"
```

and you can just `curl` the file to get ahold of it. So go wild! DDoS Github if
you want to 😉. There are plenty of ideas floating around and I'd love to see
more. Note that the header is `finsh`, *not* `finish` (so that it lines up
nicely with `start`)

#### Simply download the CSV via `curl` (`-s` to be silent, `-L` to follow redirects)
```sh
curl -sL https://github.com/beyarkay/eskom-calendar/releases/download/latest/machine_friendly.csv
area_name,stage,start,finsh,source
free-state-seretse,4,2022-09-10T10:00:00+02:00,2022-09-10T12:30:00+02:00,"https://twitter.com/Eskom_SA/status/1568494585113976835"
free-state-seretse,4,2022-09-10T18:00:00+02:00,2022-09-10T20:30:00+02:00,"https://twitter.com/Eskom_SA/status/1568494585113976835"
...
```

#### Get all data for a specific area
```sh
curl -sL https://github.com/beyarkay/eskom-calendar/releases/download/latest/machine_friendly.csv \
  | grep stellenbosch
western-cape-stellenbosch,4,2022-09-10T14:00:00+02:00,2022-09-10T16:30:00+02:00,"https://twitter.com/Eskom_SA/status/1568494585113976835"
western-cape-stellenbosch,4,2022-09-10T22:00:00+02:00,2022-09-11T00:30:00+02:00,"https://twitter.com/Eskom_SA/status/1568494585113976835"
...
```

#### Get all data for a certain day in a certain area
```sh
curl -sL https://github.com/beyarkay/eskom-calendar/releases/download/latest/machine_friendly.csv \
  | grep cape-town-area-15 \
  | grep 2022-09-11
city-of-cape-town-area-15,4,2022-09-11T00:00:00+02:00,2022-09-11T02:30:00+02:00,"https://twitter.com/Eskom_SA/status/1568494585113976835"
city-of-cape-town-area-15,4,2022-09-11T08:00:00+02:00,2022-09-11T10:30:00+02:00,"https://twitter.com/Eskom_SA/status/1568494585113976835"
...
```

#### Get the file in Python 

(mac users you might need [this](https://stackoverflow.com/a/60671292/14555505)
if you get an SSL error)

```python
import pandas as pd
url = "https://github.com/beyarkay/eskom-calendar/releases/download/latest/machine_friendly.csv"
df = pd.read_csv(url, parse_dates=['start', 'finsh'])
```

Feel free to open a PR with any other snippets or languages you think of!

## Project Goals and Alternatives

eskom-calendar tries to achieve the following goals:

- Be open-source, easy to integrate with, and encouraging of new ideas
- Provide an accesible information source for loadshedding in South Africa
- Be dead simple to use

eskom-calendar does not try to:

- Solve every solution itself. It embraces the Unix philosophy of `do one
  thing, and do it well`. Calendars are provided as an example of what's
  possible, but the heart of it is the open-source data with which websites,
  apps, bots, automations, etc, can be built.
- Compete. eskom-calendar tries to be the best product for users, but chasing
  "competitors" is distracting at best, pointless at worst.

The best known alternative would be [EskomSePush](https://sepush.co.za/), but
the author didn't want another app, and wanted to see the whole loadshedding
schedule at a glance. Hence eskom-calendar was born (making it open source was
just the default).

## Maintainers

- [Boyd Kane](https://github.com/beyarkay). Reach out on
  [twitter](https://twitter.com/beyarkay) if you want to chat in private,
  otherwise [open an
  issue](https://github.com/beyarkay/eskom-calendar/issues/new)!
- You? it's always good to have help

## Contributing

We depend on pull requests to update
[`manually_specified.yaml`](https://github.com/beyarkay/eskom-calendar/blob/main/manually_specified.yaml)
with the latest loadshedding schedule. Please [edit and submit a PR to
`manually_specified.yaml`](https://github.com/beyarkay/eskom-calendar/edit/main/manually_specified.yaml)
if [Eskom's twitter feed](https://twitter.com/Eskom_SA) announces a change.

To add a new load shedding event, do the following:
1. Click
   [here](https://github.com/beyarkay/eskom-calendar/edit/main/manually_specified.yaml)
   to edit `manually_specified.yaml`.
    
   You'll see a bunch of `changes`. Each change lists one national load
   shedding event. It includes the stage, the start time, the end time, and the
   source. The source of a change explains where the information comes from,
   and should be a link to a tweet from the [official Eskom twitter
   account](https://twitter.com/Eskom_SA). It must be included or the calendars
   will not compile.
2. Add your changes to the list. For example:

```yaml
changes:
  # This is one change, indicating loadshedding stage 1 from 5am to 4pm on the
  # 18th of July 2022
  - stage: 1
    start: 2022-07-18T05:00:00     # Timestamp is +02:00 by default
    finsh: 2022-07-18T16:00:00     # Note that finish is spelt `finsh` so it lines up with `start`
    source: https://twitter.com/Eskom_SA/status/1547189452161916928?s=20&t=2MH_-k43RExp6ISPIpi-xw
  # This is another change, indicating loadshedding stage 2 from 4pm to
  # midnight on the 18th of July 2022
  - stage: 2
    start: 2022-07-18T16:00:00
    finsh: 2022-07-18T23:59:00
    source: https://twitter.com/Eskom_SA/status/1547189452161916928?s=20&t=2MH_-k43RExp6ISPIpi-xw
  ...
```

3. Once you've added your changes, scroll to the bottom where you see `Propose
   changes` and add a commit message. The commit body must include the source
   where you got your information from.
4. Click the green button `Propose changes`
5. Check that your changes look good, and then click the button `Create pull
   request`

That's it! I'll review the PR and when I merge it, the calendars will
automatically be updated by GitHub actions.

