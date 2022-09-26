import useLocalStorage from "use-local-storage";
import {
  IAsset,
  IMachineDataResponse,
  IMyMachineData,
  IMyMachineDataGrouped,
  IProvince,
} from "./interfaces/github";
import { useEffect, useRef, useState } from "react";
import "./App.css";
import CalendarDataService from "./services/CalendarDataService";
import ThemeToggel from "./components/theme-toggel/theme-toggel";
import { Themes } from "./enums/enums";
import LoadsheddingCalendar from "./components/loadshedding-calendar/loadshedding-calendar";
import EskomCard from "./components/eskom-card/eskom-card";
import TopDownloads from "./components/top-downloads/top-downloads";

function App() {
  let calServ = useRef<CalendarDataService>(CalendarDataService.getInstance());

  const defaultDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  const ddlRef = useRef(null);
  const [theme, setTheme] = useLocalStorage(
    "theme",
    defaultDark ? Themes.Dark : Themes.Light
  );

  const toggleTheme = () => {
    if (theme == Themes.Light) {
      setTheme(Themes.Dark);
    } else {
      setTheme(Themes.Light);
    }
  };

  const [gitHubAssets, setGitHubAssets] = useState<IAsset[]>({} as IAsset[]);
  const [machineData, setMachineData] = useState<IMyMachineData[]>();

  const [provinceList, setProvinceList] = useState<IProvince[]>(
    {} as IProvince[]
  );
  const [downloadData, setDownloadData] = useState<IAsset>();
  const [assetData, setAssetData] = useState<IMyMachineDataGrouped[]>(
    {} as IMyMachineDataGrouped[]
  );
  const fetchAssets = async (e: any) => {
    var groupedAreas = await calServ.current.fetchGroupedAreaData(e);
    setDownloadData(undefined);
    setAssetData(groupedAreas.data);
  };

  const getTopDownloads = () => {
    if (gitHubAssets.length > 0) {
      var c = gitHubAssets!.filter((x) => x.download_count > 0);
      c = c
        .sort((a, b) => (a.download_count > b.download_count ? -1 : 1))
        .slice(0, 5);

      var topDownloads = (
        <div>
          <header>Top 5 downloaded files</header>
          <ul>
            {c.map((x) => {
              return (
                <li>
                  <a className=" " href={x.browser_download_url}>
                    {x.name} - {x.download_count}
                  </a>
                </li>
              );
            })}
          </ul>
        </div>
      );
      return topDownloads;
    }
  };

  useEffect(() => {
    const fetchProvinceListData = async () => {
      var d = await calServ.current.fetchProvinceList();
      setProvinceList(d);
    };

    const fetchLatestData = async () => {
      var md: IMachineDataResponse =
        await calServ.current.fetchLatestMachineData(0, 1000);
      var mdres: IMyMachineData[] = [] as IMyMachineData[];

      while (md.lastRecord !== md.totalRecords) {
        mdres.push(...md.data);
        md = await calServ.current.fetchLatestMachineData(md.lastRecord, 500);
      }
      mdres.push(...md.data);
      setMachineData(mdres);
    };

    fetchProvinceListData();
    fetchLatestData();
  }, []);

  useEffect(() => {
    if (assetData.length > 0) {
      (ddlRef.current as any).scrollIntoView(true);
    }
  }, [assetData]);

  const fetchAssetByAreaName = async (areaName: string) => {
    var data = await calServ.current.getAssetDataByCalendarName(areaName);
    setDownloadData(data);
  };

  return (
    <>
      <div className="App" data-theme={theme}>
        <div className="section-secondary-title">
          Eskom Calendar Portal
          <ThemeToggel
            currentValue={theme}
            onToggle={toggleTheme}
          ></ThemeToggel>
        </div>
        <div className="content">
          <div className="menu">
            {provinceList.length > 0 &&
              provinceList.map((x, i) => {
                return (
                  <div
                    key={i}
                    className="btns"
                    onClick={() => fetchAssets(x.key)}
                  >
                    {x.value}
                  </div>
                );
              })}
          </div>
          <div
            className={`${"main"} ${
              assetData.length > 0 ? "icsContainer" : ""
            }`}
          >
            {assetData.length > 0 && (
              <>
                <div className="filters">
                  <label>Filter </label>
                  <select
                    ref={ddlRef}
                    onChange={(e) => {
                      fetchAssetByAreaName(e.target.value);
                    }}
                  >
                    <option key={0}>Select</option>
                    {assetData.length > 0 &&
                      assetData.map((x: IMyMachineDataGrouped, i) => {
                        return (
                          <option key={i + x.area_name} value={x.area_name}>
                            {x.area_name}
                          </option>
                        );
                      })}
                  </select>
                </div>
                {downloadData && (
                  <div>
                    <LoadsheddingCalendar
                      eventCalendarName={downloadData?.name}
                    ></LoadsheddingCalendar>
                  </div>
                )}
              </>
            )}
            {downloadData && (
              <>
                <div>
                  <div className="downloadHolder">
                    Calendar file :
                    <EskomCard downloadData={downloadData} />
                  </div>
                </div>
              </>
            )}
          </div>
          <div>
            <TopDownloads></TopDownloads>
          </div>
        </div>
        <div className="footer"></div>
      </div>
    </>
  );
}

export default App;
