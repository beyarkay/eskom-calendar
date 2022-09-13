import useLocalStorage from "use-local-storage";
import { IAsset, IGitHubRelease, IProvince } from "./interfaces/github";
import { useEffect, useRef, useState } from "react";
import "./App.css";
import CalendarDataService from "./services/assets";
import ThemeToggel from "./components/theme-toggel/theme-toggel";
// import NotificationService from "./services/notificationService";
function App() {
  let calServ: CalendarDataService;
  const defaultDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  const [theme, setTheme] = useLocalStorage(
    "theme",
    defaultDark ? "dark" : "light"
  );

  const toggleTheme = () => {
    if (theme == "light") {
      setTheme("dark");
    } else {
      setTheme("light");
    }
  };

  const [gitHubData, setGitHubData] = useState<IAsset[]>({} as IAsset[]);
  const [provinceList, setProvinceList] = useState<IProvince[]>(
    {} as IProvince[]
  );
  const [downloadData, setDownloadData] = useState<IAsset>();
  const [assetData, setAssetData] = useState<IAsset[]>({} as IAsset[]);
  const fetchAssets = async (e: any) => {
    var d = gitHubData.filter((x) => {
      return x.name.indexOf(e) >= 0;
    });

    setDownloadData(undefined);
    setAssetData(await d);
  };
  useEffect(() => {
    calServ = CalendarDataService.getInstance();
    const fetchProvinceListData = async () => {
      var d = await calServ.fetchProvinceList();
      setProvinceList(d);
    };

    const fetchLatestData = async () => {
      var cc = await calServ.fetchLatest();
      setGitHubData(await cc);
    };

    fetchProvinceListData();
    fetchLatestData();
  }, []);
  return (
    <>
      <div className="App" data-theme={theme}>
        <div className="header">
          Eskom Calendar Portal{" "}
          <ThemeToggel changestuff={toggleTheme}></ThemeToggel>
        </div>
        <div className="content">
          <div className="menu">
            {provinceList.length > 0 &&
              provinceList.map((x) => {
                return (
                  <div className="card" onClick={() => fetchAssets(x.key)}>
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
            <div className="filters">
              <label>Filter </label>
              <select
                onChange={(e) => {
                  setDownloadData(
                    JSON.parse(e.target.selectedOptions[0].value)
                  );
                }}
              >
                {assetData.length > 0 &&
                  assetData.map((x: IAsset, i) => {
                    return (
                      <option key={i} value={JSON.stringify(x)}>
                        {x.town ? x.town : x.block}
                      </option>
                    );
                  })}
              </select>
            </div>
            <div>
              {downloadData && (
                <div>
                  Calendar file : 
                  <a href={downloadData.browser_download_url}>
                    {`${downloadData.name}`}
                  </a>
                </div>
              )}
            </div>
          </div>
        </div>
        <div className="footer"></div>
      </div>
    </>
  );
}

export default App;
