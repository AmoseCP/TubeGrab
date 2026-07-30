import { FORMATS, isFormatEnabled } from "../types";

interface Props {
  value: string;
  onChange: (v: string) => void;
  maxHeight: number | null;
}

export default function FormatSelect({ value, onChange, maxHeight }: Props) {
  return (
    <select
      className="rounded-lg border border-zinc-300 bg-white px-3 py-2 text-sm focus:border-blue-500 focus:outline-none"
      value={value}
      onChange={(e) => onChange(e.target.value)}
    >
      {FORMATS.map((f) => {
        const enabled = isFormatEnabled(f, maxHeight);
        return (
          <option key={f.value} value={f.value} disabled={!enabled}>
            {f.label}
            {!enabled ? "（源无此清晰度）" : ""}
          </option>
        );
      })}
    </select>
  );
}
