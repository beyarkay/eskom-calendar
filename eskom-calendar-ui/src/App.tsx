import useLocalStorage from "use-local-storage";
import { IAsset, IProvince } from "./interfaces/github";
import { useEffect, useRef, useState } from "react";
import "./App.css";
import CalendarDataService from "./services/assets";
import ThemeToggel from "./components/theme-toggel/theme-toggel";
import { Themes } from "./enums/enums";

function App() {
  let calServ: CalendarDataService;
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

  const [provinceList, setProvinceList] = useState<IProvince[]>(
    {} as IProvince[]
  );
  const [downloadData, setDownloadData] = useState<IAsset>();
  const [assetData, setAssetData] = useState<IAsset[]>({} as IAsset[]);
  const fetchAssets = async (e: any) => {
    var d = gitHubAssets.filter((x) => {
      return x.name.indexOf(e) >= 0;
    });

    setDownloadData(undefined);
    setAssetData(await d);
  };
  const getTopDownloads = () => {
    if (gitHubAssets.length > 0) {
      var c = gitHubAssets!.filter((x) => x.download_count > 0);
      c = c
        .sort((a, b) => (a.download_count > b.download_count ? -1 : 1))
        .slice(0, 5);

      var dd = (
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
      return dd;
    }
  };
  useEffect(() => {
    calServ = CalendarDataService.getInstance();
    const fetchProvinceListData = async () => {
      var d = await calServ.fetchProvinceList();
      setProvinceList(d);
    };

    const fetchLatestData = async () => {
      var cc = await calServ.fetchLatest();
      setGitHubAssets(await cc);
    };

    fetchProvinceListData();
    fetchLatestData();
  }, []);
  useEffect(() => {
    if (assetData.length > 0) {
      (ddlRef.current as any).scrollIntoView(true);
    }
  }, [assetData]);
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
              <div className="filters">
                <label>Filter </label>

                <select
                  ref={ddlRef}
                  onChange={(e) => {
                    setDownloadData(
                      JSON.parse(e.target.selectedOptions[0].value)
                    );
                  }}
                >
                  <option key={0}>Select</option>
                  {assetData.length > 0 &&
                    assetData.map((x: IAsset, i) => {
                      return (
                        <option key={i + x.name} value={JSON.stringify(x)}>
                          {x.town ? x.town : x.block}
                        </option>
                      );
                    })}
                </select>
              </div>
            )}
            {downloadData && (
              <>
                <div>
                  <div className="downloadHolder">
                    Calendar file :
                    <div className="card">
                      <div className="cardHeading">
                        <div className="imgageHolder">
                          <img src={downloadData.uploader.avatar_url} />
                        </div>
                      </div>
                      <div>
                        <label>Updated by : </label>{" "}
                        <label>{`${downloadData.uploader.login}`}</label>
                      </div>
                      <div>
                        <label>File name : </label>{" "}
                        <label>{`${downloadData.name}`}</label>
                      </div>
                      <div>
                        <label>Downloads : </label>{" "}
                        <label>{`${downloadData.download_count}`}</label>
                      </div>
                      <div className="footer">
                        <div className="btn btn-primary rounded">
                          <a href={downloadData.browser_download_url}>
                            {`Download `}
                            <i className="ti-download pr-1"></i>
                          </a>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </>
            )}
          </div>
          <div>{getTopDownloads()}</div>
        </div>
        <div className="footer"></div>
      </div>
    </>
  );
}

export default App;
