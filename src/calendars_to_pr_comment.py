from ics import Calendar
import datetime

with open("calendars/western-cape-stellenbosch.ics", "r") as f:
    c = Calendar(f.read())
    output = [
        "![Preview of loadshedding](https://github.com/beyarkay/eskom-calendar-dev/releases/download/builds/calendar_preview.png)",
        "@beyarkay here's a load shedding summary based on this PR:",
    ]
    output.append("### Load shedding for Stellenbosch")
    output.append("")
    fmt_str = "ddd D MMMM HH:mm"
    if len(c.events) == 0:
        output.append("No load shedding for Stellenbosch")
    for e in sorted(c.events):
        days = e.duration.days
        hours, rem = divmod(e.duration.seconds, 3600)
        minutes, seconds = divmod(rem, 60)
        duration_fmt = (
            "" if days == 0 else str(days) + " days, "
        ) + f"{hours}h{minutes}m"
        start = e.begin.to("Africa/Johannesburg")
        finsh = e.end.to("Africa/Johannesburg")
        lines = e.description.splitlines()
        national_ls_str = "National loadshedding information scraped from "
        source = None
        for l in lines:
            if national_ls_str in l:
                source = l.replace(national_ls_str, "")
        linked_name = (
            f"[{e.name}]({source})" if source is not None else f"{e.name} (no source)"
        )
        if start.date() == finsh.date():
            output.append(
                f"- {linked_name}: {start.format('ddd D MMMM [**]HH:mm')}-{finsh.format('HH:mm')}** (for {duration_fmt})"
            )
        else:
            output.append(
                f"- {linked_name}: **{start.format(fmt_str)} - {finsh.format(fmt_str)}** (for {duration_fmt})"
            )
    # Use `%0A` as the newline character in order to appease GitHub
    # https://github.com/orgs/community/discussions/26288#discussioncomment-3251236
    joined_output = "%0A".join(output).replace("\n", "%0A")
    print(f"::set-output name=pr-comment::{joined_output}")
