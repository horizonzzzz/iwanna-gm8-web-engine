type RuntimeHudCard = {
  id: string;
  label: string;
  value: string;
};

type RuntimeHudProps = {
  cards: RuntimeHudCard[];
};

export function RuntimeHud({ cards }: RuntimeHudProps): JSX.Element {
  return (
    <section className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
      {cards.map((card) => (
        <article
          key={card.id}
          className="rounded border border-slate-800 bg-slate-900/80 px-4 py-3"
        >
          <p className="text-xs uppercase tracking-wide text-slate-400">{card.label}</p>
          <p id={card.id} className="mt-2 text-sm text-slate-100">
            {card.value}
          </p>
        </article>
      ))}
    </section>
  );
}
