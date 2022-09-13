import { IAsset, IGitHubRelease, IProvince } from "../interfaces/github";

export default class CalendarDataService {
  private static classInstance: CalendarDataService;

  private constructor() {}

  public static getInstance(): CalendarDataService {
    if (!CalendarDataService.classInstance) {
      CalendarDataService.classInstance = new CalendarDataService();
    }

    return CalendarDataService.classInstance;
  }
  async fetchProvinceList(): Promise<IProvince[]> {
    return new Promise((resolve, reject) => {
      resolve([
        {
          key: "city-of-cape-town-area",
          value: "city of cape town area",
        },
        {
          key: "city-power",
          value: "city power",
        },
        {
          key: "eastern-cape",
          value: "eastern cape",
        },
        {
          key: "free-state",
          value: "free state",
        },
        {
          key: "limpopo",
          value: "limpopo",
        },
        {
          key: "mpumalanga",
          value: "mpumalanga",
        },
        {
          key: "north-west",
          value: "north west",
        },
        {
          key: "northern-cape",
          value: "northern cape",
        },
        {
          key: "western-cape",
          value: "western cape",
        },
      ]);
    });
  }
  async fetchLatest(): Promise<IAsset[]> {
    return fetch(
      "https://api.github.com/repos/beyarkay/eskom-calendar/releases/latest",
      {
        method: "GET",
        mode: "cors",
        headers: {
          "Content-Type": "application/json",
        },
      }
    )
      .then((x) => {
        if (x.ok) {
          return x.json();
        }
      })
      .then((d: IGitHubRelease) => {
        var dta = d.assets.map((a: IAsset) => {
          a.province = a.name
            .substring(0, a.name.lastIndexOf("-"))
            .replaceAll("-", " ");
          var tb = a.name.substring(
            a.name.lastIndexOf("-") + 1,
            a.name.lastIndexOf(".")
          );
          if (!isNaN(parseInt(tb, 10))) {
            a.block = parseInt(tb, 10);
          } else {
            a.town = tb;
          }
          return a;
        });
        return dta;
      });
  }
}
