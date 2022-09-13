import { Themes } from "../../enums/enums";
import React, { useState } from "react";
import styles from "./theme-toggel.module.css";

export interface IThemeToggle {
  onToggle: (evt?: any) => void;
  currentValue: Themes;
}

function ThemeToggel({ onToggle, currentValue }: IThemeToggle) {
  const [theme, setTheme] = useState<Themes>(currentValue);
  return (
    <div className={`${styles.ThemeToggel}`} data-testid="ThemeToggel">
      <div
        className={`${styles.button} ${
          theme == Themes.Dark ? styles.darkTheme : styles.lightTheme
        }`}
        onClick={() => {
          setTheme(theme === Themes.Dark ? Themes.Light : Themes.Dark);
          onToggle(theme === Themes.Dark ? Themes.Light : Themes.Dark);
        }}
      >
        x
      </div>
    </div>
  );
}

export default ThemeToggel;
