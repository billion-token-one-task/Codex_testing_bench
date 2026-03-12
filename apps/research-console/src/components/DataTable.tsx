export function DataTable({
  rows,
  maxRows = 80,
}: {
  rows: Array<Record<string, unknown>>;
  maxRows?: number;
}) {
  if (!rows.length) {
    return <div className="empty-box">没有可展示的数据行。</div>;
  }
  const headers = Object.keys(rows[0]);
  return (
    <div className="table-wrap">
      <table className="ledger-table">
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
    return String(value);
  }
  return JSON.stringify(value);
}
