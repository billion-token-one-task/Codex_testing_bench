export function DataTable({
  rows,
  maxRows = 80,
  compact = false,
}: {
  rows: Array<Record<string, unknown>>;
  maxRows?: number;
  compact?: boolean;
}) {
  if (!rows.length) {
    return <div className="empty-box">没有可展示的数据行。</div>;
  }
  const headers = Object.keys(rows[0]);
  return (
    <div className="table-wrap">
      <table className={`ledger-table${compact ? " ledger-table-compact" : ""}`}>
        <thead>
          <tr>
            {headers.map((header) => (
              <th key={header}>{header}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.slice(0, maxRows).map((row, rowIndex) => (
            <tr key={rowIndex}>
              {headers.map((header) => (
                <td key={header}>{formatCell(row[header])}</td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function formatCell(value: unknown) {
  if (value == null) return "—";
  if (typeof value === "string" || typeof value === "number" || typeof value === "boolean") {
    const next = String(value);
    return next.length > 180 ? `${next.slice(0, 177)}...` : next;
  }
  return JSON.stringify(value);
}
