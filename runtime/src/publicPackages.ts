export type PublicPackage = {
  /** Browser path that can be handed to `loadPackage`. */
  path: string;
  /** Directory label relative to `public/packages`, e.g. `gm8-core/IWBT_Dife`. */
  label: string;
  /** Manifest source name without the executable suffix. */
  title: string | null;
};

type ManifestShape = { source_name?: unknown };

// Every package directory under `public/packages`, resolved at transform time.
const manifests = import.meta.glob<ManifestShape>(
  ['/public/packages/*/manifest.json', '/public/packages/*/*/manifest.json'],
  { eager: true, import: 'default' }
);

export function toPublicPackage(manifestPath: string, manifest: ManifestShape): PublicPackage {
  const path = manifestPath.replace(/^\/public/, '').replace(/\/manifest\.json$/, '');
  // Some parsed manifests carry a doubled suffix (`name.exe.exe`).
  const title = typeof manifest?.source_name === 'string'
    ? manifest.source_name.replace(/(\.exe)+$/i, '').trim()
    : '';
  return {
    path,
    label: path.replace(/^\/packages\//, ''),
    title: title.length > 0 ? title : null,
  };
}

/** Packages available under `public/packages`, sorted by path. */
export const publicPackages: PublicPackage[] = Object.entries(manifests)
  .map(([manifestPath, manifest]) => toPublicPackage(manifestPath, manifest))
  .sort((left, right) => left.path.localeCompare(right.path));
