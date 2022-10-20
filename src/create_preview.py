from xml.etree import ElementTree as et
import yaml
import datetime
import os


def main():
    with open("manually_specified.yaml", "r") as f:
        raw_changes = yaml.safe_load(f)["changes"]

    changes = []
    for c in raw_changes:
        # If the change starts and ends on the same day, no worries
        if c["start"].date() == c["finsh"].date():
            changes.append(c)
            continue
        # If the change starts and ends on different days, split it up
        days = (c["finsh"].date() - c["start"].date()).days
        days = [c["start"] + datetime.timedelta(days=d) for d in range(0, days + 1)]
        for day in days:
            new_c = {k: v for k, v in c.items()}
            is_start = new_c["start"].date() == day.date()
            is_finsh = new_c["finsh"].date() == day.date()
            if is_start and is_finsh:
                start_hr = new_c["start"].hour
                start_mn = new_c["start"].minute
                start_sc = new_c["start"].second
                finsh_hr = new_c["finsh"].hour
                finsh_mn = new_c["finsh"].minute
                finsh_sc = new_c["finsh"].second
            elif is_start and not is_finsh:
                start_hr = new_c["start"].hour
                start_mn = new_c["start"].minute
                start_sc = new_c["start"].second
                finsh_hr = 23
                finsh_mn = 59
                finsh_sc = 0
            elif not is_start and is_finsh:
                start_hr = 0
                start_mn = 0
                start_sc = 0
                finsh_hr = new_c["finsh"].hour
                finsh_mn = new_c["finsh"].minute
                finsh_sc = new_c["finsh"].second
            else:
                # Not start and not finish
                start_hr = 0
                start_mn = 0
                start_sc = 0
                finsh_hr = 23
                finsh_mn = 59
                finsh_sc = 0

            new_c["start"] = datetime.datetime(
                day.year, day.month, day.day, start_hr, start_mn, start_sc
            )
            new_c["finsh"] = datetime.datetime(
                day.year, day.month, day.day, finsh_hr, finsh_mn, finsh_sc
            )
            changes.append(new_c)

    today = datetime.datetime.now()
    # Filter out all changes which occurred before today.
    changes = [c for c in changes if c["start"].date() >= today.date()]

    margin = {
        "top": 120,
        "bot": 10,
        "left": 10,
        "right": 50,
    }
    date_text_width = 85
    fontfamily = "sans-serif"
    fontsize = 14
    coct = {
        "event_color": "#69ffff",
    }
    eskom = {
        "event_color": "#3ca3ff",
    }
    eskom_color = "#ff5e5e"
    event = {"height": 85, "pad": 5, "fill": "rgb(50, 100, 100)"}

    WIDTH = 750

    HEIGHT = margin["top"] + margin["bot"]
    if len(changes) != 0:
        num_days = max((c["finsh"].date() - today.date()).days for c in changes) + 1
        HEIGHT += num_days * (event["height"] + 2 * event["pad"])

    doc = et.Element(
        "svg",
        width=str(WIDTH),
        height=str(HEIGHT),
        version="1.1",
        xmlns="http://www.w3.org/2000/svg",
    )

    def xScale(date):
        hr = date.hour
        mn = date.minute
        full_width = WIDTH - margin["left"] - margin["right"] - date_text_width
        hr_width = hr * (full_width / 24)
        mn_width = mn * (full_width / (24 * 60))
        return hr_width + mn_width

    def yScale(date):
        delta = date.date() - today.date()
        return delta.days * (event["height"] + 2 * event["pad"])

    x = WIDTH // 2
    y = 0
    text = et.Element(
        "text",
        x=str(x),
        y=str(y),
        fill="black",
        style=f"font-family:{fontfamily};font-size:{fontsize}px",
        **{"text-anchor": "middle", "dominant-baseline": "central"},
        # "text-anchor": "middle",
    )
    title_text = et.Element(
        "tspan",
        x=str(x),
        dy="1.2em",
        style=f"font-family:{fontfamily};font-size:{fontsize+20}px",
    )
    subtitle_text = et.Element("tspan", x=str(x), y=str(y), dy="5em")
    title_text.text = "eskom-calendar"
    subtitle_text.text = "Load shedding schedules in your calendar"
    text.append(title_text)
    text.append(subtitle_text)
    doc.append(text)
    if len(changes) == 0:
        with open("calendar_preview.svg", "wb") as f:
            f.write(et.tostring(doc))
        return

    # Setup the hours along the x-axis
    for i, h in enumerate(range(0, 24)):
        start = datetime.datetime(today.year, today.month, today.day, h, 0, 0)
        finsh = datetime.datetime(today.year, today.month, today.day, h, 59, 0)
        x = date_text_width + margin["left"] + xScale(start)
        width = xScale(finsh) - xScale(start)
        days = [c["finsh"] for c in changes]
        height = yScale(max(days) + datetime.timedelta(days=2))
        y = margin["top"] + fontsize
        g = et.Element("g")
        line = et.Element(
            "line",
            x1=str(x),
            y1=str(y),
            x2=str(x),
            y2=str(y + height),
            stroke="#cacaca" if i % 2 == 0 else "#ececec",
            **{"stroke-width": "2"},
        )
        g.append(line)
        text = et.Element(
            "text",
            transform=f"rotate(-90, {x}, {y})",
            x=str(x),
            y=str(y),
            dx="2",
            fill="#777" if i % 2 == 0 else "#cacaca",
            style=f"font-family:{fontfamily};font-size:{fontsize}px",
            **{"dominant-baseline": "central"},
            # "text-anchor": "middle",
        )
        text.text = start.strftime("%H:%M")
        g.append(text)
        doc.append(g)

    # Setup the dates along the y-axis
    furthest_day = (max(c["finsh"] for c in changes).date() - today.date()).days
    days = [today + datetime.timedelta(days=i) for i in range(0, furthest_day + 1)]
    for i, d in enumerate(days):
        g = et.Element("g")
        if i % 2 == 0:
            rect = et.Element(
                "rect",
                x=str(event["pad"]),
                y=str(
                    yScale(d)
                    + fontsize
                    + margin["top"]
                    - (event["pad"] if i != 0 else 0)
                ),
                width=str(WIDTH),
                height=str(
                    event["height"] + event["pad"] + (event["pad"] if i != 0 else 0)
                ),
                fill="#dbdbdb",
                opacity=str(0.5),
            )

            g.append(rect)
        text = et.Element(
            "text",
            x=str(margin["left"]),
            y=str(yScale(d) + fontsize + margin["top"]),
            dy=str(event["height"] // 2),
            style=f"font-family:{fontfamily};font-size:{fontsize}px;text-anchor:left;baseline:bottom",
        )
        text.text = d.strftime("%a %d %b")
        g.append(text)
        doc.append(g)

    # Add all the load shedding data
    for i, c in enumerate(changes):
        is_eskom = c.get("exclude", "") == "coct"
        hour_start = c["start"].hour
        hour_finsh = c["finsh"].hour
        x = date_text_width + margin["left"] + xScale(c["start"])
        y = yScale(c["start"]) + margin["top"] + fontsize
        width = xScale(c["finsh"]) - xScale(c["start"])
        ends_at_midnight = c["finsh"].hour == 23 and c["finsh"].minute == 59
        starts_too_late = c["start"].hour > 20
        if ends_at_midnight and starts_too_late:
            width += 50
        height = event["height"] // 2
        y += 0 if is_eskom else height
        g = et.Element(
            "g",
        )
        rect = et.Element(
            "rect",
            x=str(x),
            y=str(y),
            width=str(width),
            height=str(height),
            fill=eskom["event_color"] if is_eskom else coct["event_color"],
            rx="8",
            opacity=str(0.7),
        )
        dx = min(10, width // 10)
        text = et.Element(
            "text",
            x=str(x + 5),
            y=str(y),
            fill="black",
            style=f"font-family:{fontfamily};font-size:{fontsize-1}px",
        )
        stage_text = et.Element("tspan", x=str(x + 5), y=str(y), dy="1.2em")
        subtitle_text = et.Element(
            "tspan", x=str(x + 5), y=str(y), dy="2.4em", opacity=str(0.5)
        )
        stage_text.text = ("Eskom" if is_eskom else "CPT") + " Stage " + str(c["stage"])
        hr_start = c["start"].strftime("%H:%M")
        mins_to_add = (60 - c["finsh"].minute) % 60
        hr_finsh = (c["finsh"] + datetime.timedelta(minutes=mins_to_add)).strftime(
            "%H:%M"
        )
        subtitle_text.text = f"{hr_start} to {hr_finsh}"
        text.append(stage_text)
        text.append(subtitle_text)
        g.append(rect)
        g.append(text)
        doc.append(g)

    with open("calendar_preview.svg", "wb") as f:
        f.write(et.tostring(doc))


if __name__ == "__main__":
    main()
    os.system(
        "inkscape --without-gui --export-width=1500 --export-png=calendars/calendar_preview.png calendar_preview.svg"
    )
    print("Wrote calendar preview")
