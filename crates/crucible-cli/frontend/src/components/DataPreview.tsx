import { useQuery } from '@tanstack/react-query'
import { getDataPreview } from '../api/client'
import { cn } from '../lib/utils'

interface DataPreviewProps {
  /** Row indices to highlight (0-based, data rows not including header) */
  highlightedRows?: number[]
  /** Column to highlight */
  highlightedColumn?: string
}

export function DataPreview({ highlightedRows = [], highlightedColumn }: DataPreviewProps) {
  const { data, isLoading, error } = useQuery({
    queryKey: ['data-preview'],
    queryFn: getDataPreview,
  })

  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center text-muted-foreground">
        Loading data...
      </div>
    )
  }

  if (error || !data) {
    return (
      <div className="flex h-full items-center justify-center text-destructive">
        Failed to load data
      </div>
    )
  }

  const highlightedRowSet = new Set(highlightedRows)
  const highlightedColIndex = highlightedColumn
    ? data.headers.indexOf(highlightedColumn)
    : -1

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between border-b bg-muted/50 px-3 py-2">
        <span className="text-sm font-medium">Data Preview</span>
        <span className="text-xs text-muted-foreground">
          {data.truncated
            ? `Showing ${data.rows.length} of ${data.total_rows} rows`
            : `${data.total_rows} rows`}
        </span>
      </div>
      <div className="flex-1 overflow-auto">
        <table className="w-full text-sm">
          <thead className="sticky top-0 bg-muted">
            <tr>
              <th className="w-12 border-b border-r px-2 py-1.5 text-left text-xs font-medium text-muted-foreground">
                #
              </th>
              {data.headers.map((header, i) => (
                <th
                  key={header}
                  className={cn(
                    'border-b border-r px-2 py-1.5 text-left text-xs font-medium',
                    i === highlightedColIndex && 'bg-primary/10 text-primary'
                  )}
                >
                  {header}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {data.rows.map((row, rowIndex) => {
              const isHighlighted = highlightedRowSet.has(rowIndex)
              return (
                <tr
                  key={rowIndex}
                  className={cn(
                    'hover:bg-muted/50',
                    isHighlighted && 'bg-warning/20'
                  )}
                >
                  <td className="border-b border-r px-2 py-1 text-xs text-muted-foreground">
                    {rowIndex + 1}
                  </td>
                  {row.map((cell, colIndex) => (
                    <td
                      key={colIndex}
                      className={cn(
                        'border-b border-r px-2 py-1 font-mono text-xs',
                        colIndex === highlightedColIndex && 'bg-primary/5',
                        isHighlighted && colIndex === highlightedColIndex && 'bg-warning/30 font-medium'
                      )}
                    >
                      {cell || <span className="text-muted-foreground/50">-</span>}
                    </td>
                  ))}
                </tr>
              )
            })}
          </tbody>
        </table>
      </div>
    </div>
  )
}
