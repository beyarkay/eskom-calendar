import React from 'react';
import { render, screen } from '@testing-library/react';
import '@testing-library/jest-dom/extend-expect';
import TopDownloads from './top-downloads';

describe('<TopDownloads />', () => {
  test('it should mount', () => {
    render(<TopDownloads />);
    
    const topDownloads = screen.getByTestId('TopDownloads');

    expect(topDownloads).toBeInTheDocument();
  });
});