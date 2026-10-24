// Small drawings for the tour, in the app's own tokens so they follow the theme.

type Kind = "site" | "diagram" | "theme" | "export" | "assistant";

const BLOCKS = "M40 34 72 30 75 50 43 54Z M82 28 104 26 108 58 86 60Z M114 30 140 28 142 46 116 48Z M44 64 70 60 74 88 48 92Z M84 70 118 66 120 84 86 88Z M126 58 150 56 152 80 128 82Z";

export function TourArt({ kind }: { kind: Kind }) {
  return (
    <div className="tour-art" aria-hidden="true">
      <svg viewBox="0 0 190 120" width="190" height="120">
        {kind === "site" && (
          <>
            <rect x="0" y="0" width="190" height="120" className="art-ground" />
            <path d={BLOCKS} className="art-muted" />
            <circle cx="95" cy="60" r="40" className="art-site" />
            <circle cx="95" cy="60" r="2.5" className="art-brand" />
          </>
        )}
        {kind === "diagram" &&
          [0, 1, 2].map((row) => (
            <g key={row} transform={`translate(40 ${12 + row * 34})`}>
              <rect width="40" height="28" rx="3" className="art-paper" />
              <circle cx="20" cy="14" r="10" className={row === 1 ? "art-figure" : "art-muted"} />
              <rect x="52" y="12" width={row === 1 ? 60 : 46} height="4" rx="2" className={row === 1 ? "art-accent" : "art-line"} />
            </g>
          ))}
        {kind === "theme" &&
          [
            ["#f3efe6", "#2b2b2b", "#d0532c", "#9cb8c9", "#a9bf8f"],
            ["#101418", "#e8e4da", "#f2b134", "#35566e", "#3f5a3a"],
            ["#ffffff", "#4a5568", "#3b82f6", "#bfdbfe", "#bbf7d0"],
          ].map((palette, row) => (
            <g key={row} transform={`translate(35 ${16 + row * 32})`}>
              {palette.map((colour, index) => (
                <rect key={colour} x={index * 24} width="24" height="22" fill={colour} />
              ))}
              <rect width="120" height="22" rx="3" className={row === 0 ? "art-selected" : "art-outline"} />
            </g>
          ))}
        {kind === "export" && (
          <>
            {["PDF", "SVG", "PNG", "DXF"].map((label, index) => (
              <g key={label} transform={`translate(${17 + index * 40} 26)`}>
                <path d="M0 0h24l8 8v40H0z" className="art-paper" />
                <text x="16" y="34" textAnchor="middle" className="art-label">
                  {label}
                </text>
              </g>
            ))}
          </>
        )}
        {kind === "assistant" && (
          <>
            <rect x="22" y="24" width="64" height="72" rx="6" className="art-paper" />
            <rect x="32" y="38" width="44" height="4" rx="2" className="art-line" />
            <rect x="32" y="48" width="34" height="4" rx="2" className="art-line" />
            <rect x="32" y="58" width="40" height="4" rx="2" className="art-line" />
            <path d="M92 60h18" className="art-arrow" />
            <path d="M106 55l5 5-5 5" className="art-arrow" />
            <rect x="118" y="24" width="52" height="72" rx="4" className="art-paper" />
            <circle cx="144" cy="60" r="20" className="art-figure" />
            <path d="M138 54h12v12h-12z" className="art-brand" />
          </>
        )}
      </svg>
    </div>
  );
}
