/** A switch with its label, the whole row clickable. */
export function Toggle({ label, isOn, onChange }: { label: string; isOn: boolean; onChange: (isOn: boolean) => void }) {
  return (
    <label className="toggle-row">
      <span>{label}</span>
      <input type="checkbox" className="toggle" checked={isOn} onChange={(event) => onChange(event.target.checked)} />
    </label>
  );
}
