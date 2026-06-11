import { useMemo, useState } from 'react';
import type { RuntimePackage } from '../../types';

type InspectorTabsProps = {
  pkg: RuntimePackage | null;
};

const TAB_IDS = ['Package', 'Rooms', 'Objects', 'Scripts'] as const;

export function InspectorTabs({ pkg }: InspectorTabsProps): JSX.Element {
  const [activeTab, setActiveTab] = useState<(typeof TAB_IDS)[number]>('Package');

  const content = useMemo(() => {
    if (!pkg) {
      return 'Load a package to inspect runtime data.';
    }

    switch (activeTab) {
      case 'Package':
        return JSON.stringify(
          {
            source: pkg.manifest.source_name,
            engine: pkg.manifest.engine_family,
            compatibility: pkg.manifest.compatibility,
            warnings: pkg.analysis.warnings,
          },
          null,
          2
        );
      case 'Rooms':
        return JSON.stringify(pkg.rooms.slice(0, 3), null, 2);
      case 'Objects':
        return JSON.stringify(pkg.objects.slice(0, 5), null, 2);
      case 'Scripts':
        return JSON.stringify(pkg.scripts.blocks.slice(0, 5), null, 2);
    }
  }, [activeTab, pkg]);

  return (
    <section className="rounded border border-slate-800 bg-slate-950/70">
      <div role="tablist" className="flex gap-2 border-b border-slate-800 px-4 py-3">
        {TAB_IDS.map((tab) => (
          <button
            key={tab}
            role="tab"
            type="button"
            aria-selected={activeTab === tab}
            onClick={() => setActiveTab(tab)}
            className="rounded border border-slate-700 px-3 py-1.5 text-xs text-slate-100"
          >
            {tab}
          </button>
        ))}
      </div>
      <pre className="overflow-x-auto px-4 py-4 text-xs leading-6 text-slate-200">
        {content}
      </pre>
    </section>
  );
}
