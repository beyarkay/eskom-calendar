import React, { useState } from "react";
import styles from "./theme-toggel.module.css";

function ThemeToggel({ changestuff }: any) {
  const [theme, setTheme] = useState<boolean>(false);
  return (
    <div className={`${styles.ThemeToggel}`} data-testid="ThemeToggel">
      <div
        className={`${styles.button} ${
          theme == true ? styles.darkTheme : styles.lightTheme
        }`}
        onClick={() => {
          setTheme(!theme);
          changestuff();
        }}
      >
        x
      </div>
    </div>
  );
}

export default ThemeToggel;
