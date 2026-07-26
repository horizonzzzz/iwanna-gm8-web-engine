import { describe, expect, it } from 'vitest';
import { publicPackages, toPublicPackage } from './publicPackages';

describe('toPublicPackage', () => {
  it('maps a glob key to a loadable package path, label, and title', () => {
    expect(
      toPublicPackage('/public/packages/gm8-core/IWBT_Dife/manifest.json', {
        // Some parsed manifests carry a doubled `.exe` suffix.
        source_name: 'I wanna be the Dife.exe.exe',
      })
    ).toEqual({
      path: '/packages/gm8-core/IWBT_Dife',
      label: 'gm8-core/IWBT_Dife',
      title: 'I wanna be the Dife',
    });
  });

  it('falls back to a null title when the manifest has no usable source name', () => {
    expect(toPublicPackage('/public/packages/loose/manifest.json', {})).toEqual({
      path: '/packages/loose',
      label: 'loose',
      title: null,
    });
  });
});

describe('publicPackages', () => {
  // The corpus is environment-local, so only assert the shape and ordering.
  it('exposes sorted, loadable paths for whatever is checked out locally', () => {
    for (const entry of publicPackages) {
      expect(entry.path).toMatch(/^\/packages\/.+/);
      expect(entry.label).toBe(entry.path.replace('/packages/', ''));
    }
    expect(publicPackages.map((entry) => entry.path)).toEqual(
      [...publicPackages].sort((left, right) => left.path.localeCompare(right.path)).map((entry) => entry.path)
    );
  });
});
