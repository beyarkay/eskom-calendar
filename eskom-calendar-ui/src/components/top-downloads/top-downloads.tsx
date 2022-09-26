import React, { FC, useEffect, useRef, useState } from "react";
import styles from "./top-downloads.module.css";
import CalendarDataService from "../../services/CalendarDataService";
import { IAsset } from "../../interfaces/github";

function TopDownloads() {
  const calServ = useRef(CalendarDataService.getInstance());
  const [topData, setTopData] = useState({} as IAsset[]);
  useEffect(() => {
    var getData = async () => {
      var data = await calServ.current.fetchLatest();
      var c = data!.filter((x) => x.download_count > 0);
      c = c
        .sort((a, b) => (a.download_count > b.download_count ? -1 : 1))
        .slice(0, 5);
      setTopData(c);
    };
    getData();
  });
  return (
    <>
      {topData.length > 0 && (
        <div>
          <header>
            Top 5 downloaded files
            <div>
              <sub>click to Subscribe</sub>
            </div>
          </header>
          <ul>
            {topData.map((x) => {
              return (
                <li>
                  <a
                    className=" "
                    href={x.browser_download_url.replace("https:", "webcal:")}
                  >
                    {x.name} - {x.download_count}
                  </a>
                </li>
              );
            })}
          </ul>
        </div>
      )}
    </>
  );
}

export default TopDownloads;
