interface Props {
  checked: boolean;
  onChange: (v: boolean) => void;
  disabled?: boolean;
}

export function Toggle({ checked, onChange, disabled }: Props) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      disabled={disabled}
      className={`toggle${checked ? " on" : ""}${disabled ? " disabled" : ""}`}
      onClick={() => !disabled && onChange(!checked)}
    >
      <span className="toggle-thumb" />
    </button>
  );
}
