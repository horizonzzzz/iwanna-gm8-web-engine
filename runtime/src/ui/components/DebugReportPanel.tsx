import { useMemo, useState } from 'react';

type DebugReportPanelProps = {
  title: string;
  report: string;
};

export function DebugReportPanel({ title, report }: DebugReportPanelProps): JSX.Element {
  const [copied, setCopied] = useState(false);
  const lines = useMemo(() => report.split('\n'), [report]);

  async function handleCopy(): Promise<void> {
    await navigator.clipboard.writeText(report);
    setCopied(true);
    setTimeout(() => setCopied(false), 1200);
  }

  return (
    <section className="rounded border border-slate-800 bg-slate-950/70">
      <div className="flex items-center justify-between border-b border-slate-800 px-4 py-3">
        <h2 className="text-sm font-semibold text-slate-100">{title}</h2>
        <button
          type="button"
          onClick={() => void handleCopy()}
          className="rounded border border-slate-700 px-3 py-1.5 text-xs text-slate-100"
        >
          {copied ? 'Copied' : 'Copy'}
        </button>
      </div>
      <pre className="overflow-x-auto whitespace-pre-wrap px-4 py-4 text-xs leading-6 text-slate-200">
        {lines.join('\n')}
      </pre>
    </section>
  );
}
