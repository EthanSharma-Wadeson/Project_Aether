import { ReactNode, useMemo, useState } from "react";

type Column<T> = {
  key: string;
  header: string;
  render: (row: T) => ReactNode;
  sortValue?: (row: T) => string | number;
};

type Props<T> = {
  rows: T[];
  columns: Column<T>[];
  rowKey: (row: T) => string;
  searchPlaceholder?: string;
  searchFilter?: (row: T, q: string) => boolean;
  pageSize?: number;
  emptyMessage?: string;
};

export function DataTable<T>({
  rows,
  columns,
  rowKey,
  searchPlaceholder = "Search…",
  searchFilter,
  pageSize = 12,
  emptyMessage = "No rows",
}: Props<T>) {
  const [q, setQ] = useState("");
  const [page, setPage] = useState(0);

  const filtered = useMemo(() => {
    if (!q.trim() || !searchFilter) return rows;
    const needle = q.trim().toLowerCase();
    return rows.filter((r) => searchFilter(r, needle));
  }, [rows, q, searchFilter]);

  const pageCount = Math.max(1, Math.ceil(filtered.length / pageSize));
  const safePage = Math.min(page, pageCount - 1);
  const slice = filtered.slice(safePage * pageSize, safePage * pageSize + pageSize);

  return (
    <div className="data-table">
      {searchFilter && (
        <div className="toolbar">
          <input
            className="search-input"
            value={q}
            onChange={(e) => {
              setQ(e.target.value);
              setPage(0);
            }}
            placeholder={searchPlaceholder}
            aria-label="Search table"
          />
          <span className="muted">
            {filtered.length} result{filtered.length === 1 ? "" : "s"}
          </span>
        </div>
      )}
      <div className="table-wrap">
        <table>
          <thead>
            <tr>
              {columns.map((c) => (
                <th key={c.key}>{c.header}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {slice.length === 0 ? (
              <tr>
                <td colSpan={columns.length} className="muted">
                  {emptyMessage}
                </td>
              </tr>
            ) : (
              slice.map((row) => (
                <tr key={rowKey(row)}>
                  {columns.map((c) => (
                    <td key={c.key}>{c.render(row)}</td>
                  ))}
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
      {pageCount > 1 && (
        <div className="pager">
          <button
            type="button"
            className="secondary"
            disabled={safePage === 0}
            onClick={() => setPage((p) => Math.max(0, p - 1))}
          >
            Previous
          </button>
          <span className="muted">
            Page {safePage + 1} / {pageCount}
          </span>
          <button
            type="button"
            className="secondary"
            disabled={safePage >= pageCount - 1}
            onClick={() => setPage((p) => Math.min(pageCount - 1, p + 1))}
          >
            Next
          </button>
        </div>
      )}
    </div>
  );
}
