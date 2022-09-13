import React from 'react';
import { render, screen } from '@testing-library/react';
import '@testing-library/jest-dom/extend-expect';
import ThemeToggel from './theme-toggel';


describe('<ThemeToggel />', () => {
  test('it should mount', () => {
    render(<ThemeToggel />);
    
    const themeToggel = screen.getByTestId('ThemeToggel');

    expect(themeToggel).toBeInTheDocument();
  });
});