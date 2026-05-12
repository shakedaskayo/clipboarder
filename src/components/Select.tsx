interface Option {
  value: string | number;
  label: string;
}

interface Props {
  value: string | number;
  options: Option[];
  onChange: (v: string | number) => void;
}

export function Select({ value, options, onChange }: Props) {
  return (
    <div className="select">
      <select
        value={String(value)}
        onChange={e => {
          const raw = e.target.value;
          const match = options.find(o => String(o.value) === raw);
          if (match) onChange(match.value);
        }}
      >
        {options.map(o => (
          <option key={String(o.value)} value={String(o.value)}>{o.label}</option>
        ))}
      </select>
      <svg className="select-chev" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
        <polyline points="6 9 12 15 18 9" />
      </svg>
    </div>
  );
}
