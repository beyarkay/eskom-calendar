import React from 'react';
import { render, screen } from '@testing-library/react';
import '@testing-library/jest-dom/extend-expect';
import LoadsheddingCalendar from './loadshedding-calendar';

describe('<LoadsheddingCalendar />', () => {
  test('it should mount', () => {
    render(<LoadsheddingCalendar />);
    
    const loadsheddingCalendar = screen.getByTestId('LoadsheddingCalendar');

    expect(loadsheddingCalendar).toBeInTheDocument();
  });
});