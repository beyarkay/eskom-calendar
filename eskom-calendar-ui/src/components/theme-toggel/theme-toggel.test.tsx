import React from "react";
import { render, screen } from "@testing-library/react";
import "@testing-library/jest-dom/extend-expect";
import ThemeToggel from "./theme-toggel";
import { Themes } from "../../enums/enums";

describe("<ThemeToggel />", () => {
  test("it should mount", () => {
    render(<ThemeToggel currentValue={Themes.Dark} onToggle={() => {}} />);

    const themeToggel = screen.getByTestId("ThemeToggel");

    expect(themeToggel).toBeInTheDocument();
  });
});
