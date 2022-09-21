import React from "react";
import { render, screen } from "@testing-library/react";
import "@testing-library/jest-dom/extend-expect";
import EskomCard from "./eskom-card";
import { IAsset } from "../../interfaces/github";

describe("<EskomCard />", () => {
  test("it should mount", () => {
    render(<EskomCard downloadData={{} as IAsset} />);

    const eskomCard = screen.getByTestId("EskomCard");

    expect(eskomCard).toBeInTheDocument();
  });
});
