import {
  IAsset,
  IGitHubRelease,
  IMachineDataResponse,
  IMyMachineDataGroupedResponse,
  IProvince,
  ISuburbData,
} from "../interfaces/github";

export default class CalendarDataService {
  private static classInstance: CalendarDataService;
  static calendarBaseUrl =(process.env.REACT_APP_CALENDAR_BASE_URL as string) // "https://localhost:44373/api/Calendar/";
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
        { key: "city-of-cape-town", value: "city of cape town" },
        { key: "city-power", value: "city power" },
        { key: "eastern-cape", value: "eastern cape" },
        { key: "free-state", value: "free state" },
        { key: "kwazulu-natal", value: "kwazulu natal" },
        { key: "gauteng", value: "gauteng" },
        { key: "limpopo", value: "limpopo" },
        { key: "mpumalanga", value: "mpumalanga" },
        { key: "north-west", value: "north west" },
        { key: "northern-cape", value: "northern cape" },
        { key: "western-cape", value: "western cape" },
      ]);
    });
  }
  async fetchGroupedAreaData(
    area_name: string
  ): Promise<IMyMachineDataGroupedResponse> {
    return fetch(CalendarDataService.calendarBaseUrl + "GetDistinctAreas?areaName=" + area_name, {
      method: "GET",
      mode: "cors",
      headers: {
        "Content-Type": "application/json",
      },
    }).then((x) => {
      if (x.ok) {
        return x.json();
      }
    });
  }

  async getAssetDataByCalendarName(calendarName: string) {
    debugger
    return fetch(
      CalendarDataService.calendarBaseUrl + "GetAssetByCalendarName?calendarname=" + calendarName,
      {
        method: "GET",
        mode: "cors",
        headers: {
          "Content-Type": "application/json",
        },
      }
    ).then((x) => {
      if (x.ok) {
        return x.json();
      }
    });
  }

  async fetchLatestMachineData(
    lastRecord: number,
    recordsToRetrieve: number
  ): Promise<IMachineDataResponse> {
    return fetch(
      CalendarDataService.calendarBaseUrl + "GetMachineFriendlyInfo?lastRecord=" +
        lastRecord +
        "&recordsToRetrieve=" +
        recordsToRetrieve,
      {
        method: "GET",
        mode: "cors",
        headers: {
          "Content-Type": "application/json",
        },
      }
    ).then((x) => {
      if (x.ok) {
        return x.json();
      }
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
          // We need to find a better way to sanitize the data
          // so that we can split between province and suburb/blocks
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

  async fetchNewMachineFileData(areaName: string): Promise<any> {
    return fetch(
      CalendarDataService.calendarBaseUrl + "GetDataByArea?areaName=" +
        areaName,
      {
        method: "GET",
        mode: "cors",
        headers: {
          "Content-Type": "application/json",
        },
      }
    ).then((x) => {
      if (x.ok) {
        return x.json();
      }
    });
  }

  async fetchSuburbs(calendarName: string): Promise<ISuburbData[]> {
    return fetch(
      CalendarDataService.calendarBaseUrl + "GetCalendarSuburbs?calendarName=" +
        calendarName,
      {
        method: "GET",
        mode: "cors",
        headers: {
          "Content-Type": "application/json",
        },
      }
    ).then((x) => {
      if (x.ok) {
        return x.json();
      }
    });
  }
}
