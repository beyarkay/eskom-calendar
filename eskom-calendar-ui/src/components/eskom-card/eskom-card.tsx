import { useEffect, useState } from "react";
import CalendarDataService from "../../services/CalendarDataService";
import { IAsset } from "../../interfaces/github";
import styles from "./eskom-card.module.css";

interface EskomCardProps {
  downloadData: IAsset;
}
function EskomCard({ downloadData }: EskomCardProps) {
  let calServ: CalendarDataService;
  const [suburbData, setSuburbData] = useState<any>();
  useEffect(() => {
    calServ = CalendarDataService.getInstance();
    const getSubs = async () => {
      var d = await calServ.fetchSuburbs(downloadData.name);
      setSuburbData(d);
    };
    getSubs();
  }, [downloadData]);
  return (
    <>
      <div className={styles.card}>
        <div className={styles.cardHeading}>
          <div className={styles.imgageHolder}>
            <img src={downloadData.uploader.avatar_url} />
          </div>
        </div>
        <div>
          <label>Updated by : </label>{" "}
          <label>{`${downloadData.uploader.login}`}</label>
        </div>
        <div>
          <label>File name : </label> <label>{`${downloadData.name}`}</label>
        </div>
        <div>
          <label>Downloads : </label>{" "}
          <label>{`${downloadData.download_count}`}</label>
        </div>
        <div className={styles.footer}>
          <div className="btn btn-primary rounded">
            <a href={downloadData.browser_download_url}>
              {`Download `}
              <i className="ti-download pr-1"></i>
            </a>
          </div>
        </div>
      </div>
      <div className={styles.suburbList}>
        {suburbData &&
          suburbData.map((x: any, i: number) => {
            return (
              <div className={styles.suburb} key={i}>
                {x.subName}
              </div>
            );
          })}
      </div>
    </>
  );
}
export default EskomCard;
