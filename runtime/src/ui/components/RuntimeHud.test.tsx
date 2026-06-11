import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import { RuntimeHud } from './RuntimeHud';

describe('RuntimeHud', () => {
  it('renders the preserved runtime telemetry cards', () => {
    render(
      <RuntimeHud
        cards={[
          { id: 'runtime-status', label: 'Status', value: 'WASM runtime active' },
          { id: 'runtime-room', label: 'Room', value: '143: sampleroom01' },
          { id: 'runtime-tick', label: 'Tick', value: 'Tick: 42' },
          { id: 'runtime-player', label: 'Player', value: 'Player: x=12' },
          { id: 'runtime-input', label: 'Input', value: 'Input: keys=[16,39]' },
          { id: 'runtime-events', label: 'Events', value: 'runtime-instance-created' },
          { id: 'runtime-diagnostics', label: 'Diagnostics', value: 'Diagnostics: 2 recent' },
          { id: 'runtime-performance', label: 'Frame', value: 'Frame: 12.4ms' },
        ]}
      />
    );

    expect(screen.getByText('WASM runtime active')).toBeInTheDocument();
    expect(screen.getByText('143: sampleroom01')).toBeInTheDocument();
    expect(screen.getByText('Frame: 12.4ms')).toBeInTheDocument();
  });
});
