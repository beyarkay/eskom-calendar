from typing_extensions import deprecated
import tqdm
import yaml
import psycopg2
import pandas as pd
from enum import Enum
from typing import NewType, cast
import datetime
from typing import Optional
import dataclasses
import random
import sys
import subprocess

with open("area_metadata.yaml", "r") as f:
    data = yaml.safe_load(f)['area_details']


Id = NewType("Id", str)
MunicName = NewType("MunicName", str)

ids = set()


def gen_id(hint=None) -> Id:
    alphabet = '0123456789abcdefghijklmnopqrstuvwxyz'
    if hint is None:
        choices = random.choices(alphabet, k=6)
        _id = Id(''.join(choices[:3]) + '-' + ''.join(choices[3:]))
    else:
        _id = ''.join(c for c in hint.lower() if (c.isascii() and c.isalnum()))
        if len(_id) > 7:
            _id = (
                _id
                .replace("cityofcapetownarea", 'cpt')
                .replace("cityofcapetown", 'cpt')
                .replace("gautengekurhuleniblock", 'ekurhuleni')
                .replace("kwazulunatalethekwiniblock", 'ethekwini')
                .replace("gautengemfuleniarea", 'emfuleni')
                .replace("gautengtshwanegroup", 'tshwane')
                .replace("citypower", 'cp')
                .replace("buffalocityblock19", 'buffalocity')
                .replace("eskomdirect", "eskm")
                .replace("even", "e")
                .replace("odd", "o")
                .replace("cityof", "")
                .replace("easterncape", "ec")
                .replace("freestate", "fs")
                .replace("gauteng", "gp")
                .replace("kwazulunatal", "kzn")
                .replace("limpopo", "lp")
                .replace("mpumalanga", "mp")
                .replace("northerncape", "nc")
                .replace("northwest", "nw")
                .replace("westerncape", "wc")

            )
        if len(_id) > 7:
            _id = _id.replace('a', '').replace('e', '').replace(
                'i', '').replace('o', '').replace('u', '')
        if len(_id) > 7:
            _id = _id[:7]
        _id = Id(_id)

    while _id in ids:
        print(f"{_id} in ids, hint was {hint}")
        choices = random.choices(alphabet, k=6)
        _id = Id(''.join(choices[:3]) + '-' + ''.join(choices[3:]))
    ids.add(_id)
    return _id


UrlId = NewType("UrlId", Id)
_RecurringScheduleId = NewType("_RecurringScheduleId", Id)
_AreaId = NewType("_AreaId", Id)
MunicipalityId = NewType("MunicipalityId", Id)
AliasId = NewType("AliasId", Id)
ScheduleId = NewType("ScheduleId", Id)
PlaceId = NewType("PlaceId", Id)
GeofenceId = NewType("GeofenceId", Id)

AreaName = str | list[str]


class MunicipalityKind(Enum):
    Metropolitan = 1,
    District = 2,
    Local = 3,


class Province(Enum):
    EasternCape = 1,
    FreeState = 2,
    Gauteng = 3,
    KwazuluNatal = 4,
    Limpopo = 5,
    Mpumalanga = 6,
    NorthWest = 7,
    NorthernCape = 8,
    WesternCape = 9,


def str_to_prov(s: Optional[str]) -> Optional[Province]:
    if s is None:
        return None
    match s.replace("-", "").replace("_", ""):
        case "easterncape": return Province.EasternCape  # noqa: E701
        case "freestate": return Province.FreeState  # noqa: E701
        case "gauteng": return Province.Gauteng  # noqa: E701
        case "kwazulunatal": return Province.KwazuluNatal  # noqa: E701
        case "limpopo": return Province.Limpopo  # noqa: E701
        case "mpumalanga": return Province.Mpumalanga  # noqa: E701
        case "northwest": return Province.NorthWest  # noqa: E701
        case "northerncape": return Province.NorthernCape  # noqa: E701
        case "westerncape": return Province.WesternCape  # noqa: E701
    raise Exception(f"Boo hoo {s}")


@dataclasses.dataclass
class Url:
    _id: UrlId
    url: str


@dataclasses.dataclass
class Schedule:
    _id: ScheduleId
    filename: str
    sources_id: Optional[str]
    info_id: Optional[str]
    last_updated: Optional[datetime.datetime]
    valid_from: Optional[datetime.datetime]
    valid_until: Optional[datetime.datetime]


@dataclasses.dataclass
class Municipality:
    _id: MunicipalityId
    name: str
    province: Province
    kind: Optional[MunicipalityKind]


@dataclasses.dataclass
class Alias:
    _id: AliasId
    alias: str


@dataclasses.dataclass
class Place:
    _id: PlaceId
    schedule_id: ScheduleId
    display_name: str
    munic_id: Optional[MunicipalityId]
    name_aliases_id: Optional[AliasId]


@dataclasses.dataclass
class Geofence:
    _id: GeofenceId
    lat: float
    lng: float
    path_index: int
    point_index: int
    place_id: PlaceId


@dataclasses.dataclass
class Area:
    name: AreaName
    province: Optional[str]
    municipality: Optional[str]


@dataclasses.dataclass
class AreaDetail:
    calendar_name: str
    provider: str
    province: Optional[str]
    municipality: Optional[str]
    city: Optional[str]
    source: str
    source_info: str
    areas: list[Area]


@deprecated
@dataclasses.dataclass
class NewRecurringSchedule:
    _id: _RecurringScheduleId
    filename: str
    sources_id: UrlId
    info_id: UrlId
    last_updated: Optional[datetime.datetime]
    valid_from: Optional[datetime.datetime]
    valid_until: Optional[datetime.datetime]


@deprecated
@dataclasses.dataclass
class NewArea:
    _id: _AreaId
    name: Optional[str]
    schedule_id: _RecurringScheduleId
    alias_id: Optional[AliasId]
    province: Optional[Province]
    munic: Optional[MunicName]


urls: list[Url] = []
schedules: list[Schedule] = []
aliases: list[Alias] = []
places: list[Place] = []
geofences: list[Geofence] = []

known_munics: pd.DataFrame = cast(
    pd.DataFrame, pd.read_csv("municipalities.csv"))
# Assign all municipalities an approximately mnemonic ID
known_munics['_id'] = known_munics['name'].apply(
    lambda name: gen_id(hint=name))

for i, d in tqdm.tqdm(enumerate(data), total=len(data)):
    # print(f'{i}/{len(data)}')
    area_detail = AreaDetail(
        **({'province': None, 'municipality': None, 'city': None} | d)
    )

    area_detail.areas = [Area(
        **({'province': None, 'municipality': None} | a)) for a in area_detail.areas
    ]

    info_id = UrlId(gen_id())
    recsched_id = _RecurringScheduleId(gen_id())

    # Get all URLs which have already been seen and are equal to the source of
    # the current area_detail
    u = [url for url in urls if url.url == area_detail.source]
    # If this URL hasn't been seen before, add it to the list of unique URLs
    # and assign it as the source_id
    if len(u) == 0:
        sources_id = UrlId(gen_id())
        urls.append(Url(sources_id, area_detail.source))
    elif len(u) == 1:
        sources_id = u[0]._id
    else:
        raise Exception(f"Unreachable: {area_detail.source}, {u}")

    # repeat the same as above, but for the info
    u = [url for url in urls if url.url == area_detail.source_info]
    if len(u) == 0:
        info_id = UrlId(gen_id())
        urls.append(Url(info_id, area_detail.source_info))
    elif len(u) == 1:
        info_id = u[0]._id
    else:
        raise Exception(f"Unreachable: {area_detail.source_info}, {u}")

    # Get the file creation date and use it as the `valid_from` date
    filename = area_detail.calendar_name.replace('.ics', '.csv')
    result = subprocess.run([
        'git',
        'log',
        '--diff-filter=A',
        '--',
        f'generated/{filename}'
    ], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=True)
    # The commit log looks like:
    # commit 237d6d9df20059133f89fe2d40b93417075fb43e
    # Author: beyarkay <boydrkane@gmail.com>
    # Date:   Sat Jul 16 22:01:17 2022 +0200
    #
    #     Add code to parse COCT html files
    #
    creation_log = result.stdout.split('\n')[2].replace("Date:", "").strip()
    valid_from = datetime.datetime.strptime(
        creation_log,  "%a %b %d %H:%M:%S %Y %z")

    # get the file last updated date and use it as the `last_updated` date
    result = subprocess.run([
        'git',
        'log',
        '--follow',
        '--format=%ad',
        '--',
        f'generated/{filename}'
    ], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=True)
    last_updated_log = result.stdout.split('\n')[0]
    last_updated = datetime.datetime.strptime(
        last_updated_log,  "%a %b %d %H:%M:%S %Y %z")

    filename = area_detail.calendar_name.replace(".ics", "")
    schedule_id = ScheduleId(gen_id(hint=filename))
    schedules.append(Schedule(
        _id=schedule_id,
        filename=filename,
        sources_id=sources_id,
        info_id=info_id,
        last_updated=last_updated,
        valid_from=valid_from,
        valid_until=None,
    ))

    for i, area in enumerate(area_detail.areas):
        # Figure out the province
        province = str_to_prov(area.province)

        # Figure out the municipality ID
        munic_name = None
        municipality_id = None
        if area.municipality is not None:
            # Figure out the name of the municipality
            def norm_munic(m):
                return (
                    m.lower()
                    .replace(" local municipality", "")
                    .replace(" district municipality", "")
                    .replace(" metropolitan municipality", "")
                    .replace(" ", "_")
                    .replace("-", "_")
                    .replace("!", "")
                    .replace("é", "e")
                    .replace("â", "a")
                )
            normed_needle = norm_munic(area.municipality)
            matches = known_munics[
                known_munics['name'].apply(norm_munic) == normed_needle
            ]
            assert matches is not None
            if len(matches) != 1:
                print(f"{area.municipality} is not in the known munics, {matches}")
            else:
                munic_name = matches['name'].values[0]
                municipality_id = matches['_id'].values[0]

        # Handle scalar names
        if type(area.name) is str:
            # These assertions are true but not covered by the type system ):
            assert area.municipality is None
            assert area.province is None

            places.append(Place(
                _id=PlaceId(gen_id(hint=area.name)),
                schedule_id=schedule_id,
                display_name=area.name,
                munic_id=municipality_id,
                name_aliases_id=None,  # Scalar names have no aliases
            ))

        elif type(area.name) is list:
            # Handle lists of names / lists of objects
            alias_id = AliasId(gen_id())
            for item in area.name:
                assert type(item) is str, f"{item} isn't a string ):"
                places.append(Place(
                    _id=PlaceId(gen_id(hint=item)),
                    schedule_id=schedule_id,
                    display_name=item,
                    munic_id=municipality_id,
                    name_aliases_id=None,
                ))

        else:
            raise Exception(
                "Type {type(area.name)} of `{area.name}` not known")


urls_df = pd.DataFrame(urls)
schedules_df = pd.DataFrame(schedules)
places_df = pd.DataFrame(places)
aliases_df = pd.DataFrame(aliases)


def conn_to_db():
    try:
        conn = psycopg2.connect(
            database="eskom-calendar",
            user="brk",
        )
    except psycopg2.Error as e:
        print("Error: Unable to connect to the database")
        print(e)
        sys.exit(1)
    return conn


conn = conn_to_db()
cursor = conn.cursor()

db_ids = []
for i, row in tqdm.tqdm(urls_df.iterrows(), total=len(urls_df)):
    insert_query = """INSERT INTO urls (url) VALUES (%s) RETURNING id"""
    data_to_insert = (row['url'],)
    cursor.execute(insert_query, data_to_insert)
    inserted_id = cursor.fetchone()[0]
    db_ids.append(inserted_id)
    conn.commit()
urls_df['db_id'] = db_ids

for i, row in tqdm.tqdm(schedules_df.iterrows(), total=len(schedules_df)):
    insert_query = """INSERT INTO schedules (id, filename, sources_id, info_id, last_updated, valid_from, valid_until)
                    VALUES (%s, %s, %s, %s, %s, %s, %s)"""
    info_id = urls_df.loc[
        urls_df['_id'] == row['info_id'],
        'db_id'
    ].values[0]
    sources_id = urls_df.loc[
        urls_df['_id'] == row['sources_id'],
        'db_id'
    ].values[0]
    data_to_insert = (
        row['_id'],
        row['filename'],
        int(sources_id),
        int(info_id),
        row['last_updated'],
        row['valid_from'],
        row['valid_until'],
    )
    cursor.execute(insert_query, data_to_insert)
    conn.commit()

for i, row in tqdm.tqdm(known_munics.iterrows(), total=len(known_munics)):
    insert_query = """INSERT INTO municipalities (id, name, province, kind)
                    VALUES (%s, %s, %s, %s)"""
    data_to_insert = (
        row['_id'],
        row['name'],
        row['province'],
        row['kind'],
    )
    cursor.execute(insert_query, data_to_insert)
    conn.commit()

for i, row in tqdm.tqdm(places_df.iterrows(), total=len(places_df)):
    # print(f'{i}/{len(places_df)}: {row["display_name"]}')
    insert_query = """INSERT INTO places (id, schedule_id, display_name, munic_id, name_aliases_id)
                    VALUES (%s, %s, %s, %s, %s)"""
    data_to_insert = (
        row['_id'],
        row['schedule_id'],
        row['display_name'].title(),
        row['munic_id'],
        row['name_aliases_id'],
    )
    cursor.execute(insert_query, data_to_insert)
    conn.commit()


print(r"""
-- To export from postgres into CSV files
\copy aliases        TO 'data/aliases.csv' WITH (FORMAT CSV, HEADER)
\copy geofences      TO 'data/geofences.csv' WITH (FORMAT CSV, HEADER)
\copy municipalities TO 'data/municipalities.csv' WITH (FORMAT CSV, HEADER)
\copy places         TO 'data/places.csv' WITH (FORMAT CSV, HEADER)
\copy schedules      TO 'data/schedules.csv' WITH (FORMAT CSV, HEADER)
\copy urls(url)      TO 'data/urls.csv' WITH (FORMAT CSV, HEADER)
""")
