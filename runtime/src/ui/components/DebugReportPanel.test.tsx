import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { DebugReportPanel } from './DebugReportPanel';

describe('DebugReportPanel', () => {
  it('renders the report and copies the full text', async () => {
    const writeText = vi.fn(async () => undefined);
    Object.assign(navigator, {
      clipboard: { writeText },
    });

    render(
      <DebugReportPanel
        title="Debug Report"
        report={'Status: ok\nRoom: 143 sampleroom01'}
      />
    );

    fireEvent.click(screen.getByRole('button', { name: 'Copy' }));

    expect(writeText).toHaveBeenCalledWith('Status: ok\nRoom: 143 sampleroom01');
    expect(screen.getByText((_content, element) => element?.tagName === 'PRE' && element.textContent?.includes('Status: ok') === true)).toBeInTheDocument();
  });
});
