import yaml
import pandas as pd
import uuid
from enum import Enum
from typing import NewType
import datetime
from typing import Optional
import dataclasses

with open("area_metadata.yaml", "r") as f:
    data = yaml.safe_load(f)['area_details']


UrlId = NewType("UrlId", uuid.UUID)
RecurringScheduleId = NewType("RecurringScheduleId", uuid.UUID)
AreaId = NewType("AreaId", uuid.UUID)
MunicipalityId = NewType("MunicipalityId", uuid.UUID)
AliasId = NewType("AliasId", uuid.UUID)

# TODO normalize this
AreaName = str | list[str]


class MunicipalityType(Enum):
    Metropolitan = 1,
    District = 2,
    Local = 3,
    Unknown = 4,


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
    match s:
        case None: return None  # noqa: E701
        case "eastern-cape": return Province.EasternCape  # noqa: E701
        case "free-state": return Province.FreeState  # noqa: E701
        case "gauteng": return Province.Gauteng  # noqa: E701
        case "kwazulu-natal": return Province.KwazuluNatal  # noqa: E701
        case "limpopo": return Province.Limpopo  # noqa: E701
        case "mpumalanga": return Province.Mpumalanga  # noqa: E701
        case "north-west": return Province.NorthWest  # noqa: E701
        case "northern-cape": return Province.NorthernCape  # noqa: E701
        case "western-cape": return Province.WesternCape  # noqa: E701
    raise Exception(f"Boo hoo {s}")


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


@dataclasses.dataclass
class NewRecurringSchedule:
    _id: RecurringScheduleId
    filename: str
    sources_id: UrlId
    info_id: UrlId
    last_updated: Optional[datetime.datetime]
    valid_from: Optional[datetime.datetime]
    valid_until: Optional[datetime.datetime]


@dataclasses.dataclass
class Alias:
    _id: AliasId
    alias: str


@dataclasses.dataclass
class Municipality:
    _id: MunicipalityId
    _type: MunicipalityType
    name: str


@dataclasses.dataclass
class Url:
    _id: UrlId
    url: str


@dataclasses.dataclass
class NewArea:
    _id: AreaId
    name: Optional[str]
    schedule_id: RecurringScheduleId
    alias_id: AliasId
    province: Optional[Province]
    munic_id: Optional[MunicipalityId]


recurring_schedules: list[NewRecurringSchedule] = []
urls: list[Url] = []
areas: list[NewArea] = []
aliases: list[Alias] = []
munics: list[Municipality] = []

known_munics: pd.DataFrame = pd.read_csv("data/municipalities.csv")

for d in data:
    area_detail = AreaDetail(
        **({'province': None, 'municipality': None, 'city': None} | d)
    )

    area_detail.areas = [Area(
        **({'province': None, 'municipality': None} | a)) for a in area_detail.areas
    ]

    info_id = UrlId(uuid.uuid4())
    recsched_id = RecurringScheduleId(uuid.uuid4())

    u = [url for url in urls if url.url == area_detail.source]
    if len(u) == 0:
        sources_id = UrlId(uuid.uuid4())
        urls.append(Url(sources_id, area_detail.source))
    else:
        sources_id = u[0]._id

    u = [url for url in urls if url.url == area_detail.source_info]
    if len(u) == 0:
        info_id = UrlId(uuid.uuid4())
        urls.append(Url(info_id, area_detail.source_info))
    else:
        info_id = u[0]._id

    recurring_schedules.append(NewRecurringSchedule(
        _id=recsched_id,
        filename=area_detail.calendar_name,
        sources_id=sources_id,
        info_id=info_id,
        last_updated=None,
        valid_from=None,
        valid_until=None
    ))

    for i, area in enumerate(area_detail.areas):
        # Figure out the province
        province = str_to_prov(area.province)

        # Figure out the municipality ID
        munic_id = None
        if area.municipality is not None:
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
            matches = known_munics[known_munics['name'].apply(
                norm_munic) == normed_needle]
            if len(matches) == 0:  # noqa: E501
                print(f"{area.municipality} is not in the known munics")
            else:
                munic_id = MunicipalityId(matches['_id'].values[0])

        # Handle scalar names
        if type(area.name) is str:
            # These assertions are true but not covered by the type system ):
            assert area.municipality is None
            assert area.province is None

            alias_id = AliasId(uuid.uuid4())
            # Add the name as an alias. Since area.name is a string, we only
            # have one alias.
            aliases.append(Alias(
                _id=alias_id,
                alias=area.name
            ))

            areas.append(NewArea(
                _id=AreaId(uuid.uuid4()),
                name=area.name,
                schedule_id=recsched_id,
                alias_id=alias_id,
                province=province,
                munic_id=munic_id,
            ))

        elif type(area.name) is list:
            # Handle lists of names / lists of objects
            alias_id = AliasId(uuid.uuid4())
            for item in area.name:
                assert type(item) is str, f"{item} isn't a string ):"
                aliases.append(Alias(
                    _id=alias_id,
                    alias=item
                ))

            areas.append(NewArea(
                _id=AreaId(uuid.uuid4()),
                # If there are loads of names, how can we pick one?
                name=None,
                schedule_id=recsched_id,
                alias_id=alias_id,
                province=province,
                munic_id=munic_id,
            ))
        else:
            raise Exception(
                "Type {type(area.name)} of `{area.name}` not known")


areas_df = pd.DataFrame(areas)
recurring_schedules_df = pd.DataFrame(recurring_schedules)
urls_df = pd.DataFrame(urls)
aliases_df = pd.DataFrame(aliases)

areas_df.to_csv("data/areas.csv", index=False)
recurring_schedules_df.to_csv("data/recurring_schedules.csv", index=False)
urls_df.to_csv("data/urls.csv", index=False)
aliases_df.to_csv("data/aliases.csv", index=False)


print("""
TODO: Remove munic_id, make it just one enum
Make the file size smaller.
Are aliases really needed?
""")
