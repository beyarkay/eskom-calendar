import { IAsset } from "../../interfaces/github";
import styles from "./eskom-card.module.css";

interface EskomCardProps {
  downloadData: IAsset;
}
function EskomCard({ downloadData }: EskomCardProps) {
  debugger;
  return (
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
  );
}
export default EskomCard;
