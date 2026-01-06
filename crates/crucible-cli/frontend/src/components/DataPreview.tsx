import { useRef, useState, useCallback, useEffect } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useVirtualizer } from '@tanstack/react-virtual'
import { getDataPreview } from '../api/client'
import { cn } from '../lib/utils'

interface DataPreviewProps {
  /** Row indices to highlight (0-based, data rows not including header) */
  highlightedRows?: number[]
  /** Column to highlight */
  highlightedColumn?: string
}

/** Number of rows to fetch per page */
const PAGE_SIZE = 100
/** Estimated row height in pixels */
const ROW_HEIGHT = 28

export function DataPreview({ highlightedRows = [], highlightedColumn }: DataPreviewProps) {
  const queryClient = useQueryClient()
  const parentRef = useRef<HTMLDivElement>(null)

  // Fetch the initial page (offset 0)
  const { data, isLoading, error } = useQuery({
    queryKey: ['data-preview', 0],
    queryFn: () => getDataPreview({ offset: 0, limit: PAGE_SIZE }),
  })

  // Cache all fetched rows for rendering
  const [allRows, setAllRows] = useState<string[][]>([])
  const [headers, setHeaders] = useState<string[]>([])
  const [totalRows, setTotalRows] = useState(0)

  // Update cached data when new data arrives
  useEffect(() => {
    if (data) {
      setHeaders(data.headers)
      setTotalRows(data.total_rows)

      // Merge new rows into cache
      setAllRows(prev => {
        const newRows = [...prev]
        // Ensure array is long enough
        while (newRows.length < data.offset + data.rows.length) {
          newRows.push([])
        }
        // Insert fetched rows at their correct positions
        for (let i = 0; i < data.rows.length; i++) {
          newRows[data.offset + i] = data.rows[i]
        }
        return newRows
      })
    }
  }, [data])

  // Virtual scrolling setup
  const rowVirtualizer = useVirtualizer({
    count: totalRows,
    getScrollElement: () => parentRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 10,
  })

  // Fetch more data as user scrolls
  const fetchedRanges = useRef(new Set<number>())

  const handleScroll = useCallback(() => {
    const virtualItems = rowVirtualizer.getVirtualItems()
    if (virtualItems.length === 0) return

    const firstVisible = virtualItems[0].index
    const lastVisible = virtualItems[virtualItems.length - 1].index

    // Calculate which pages we need
    const firstPage = Math.floor(firstVisible / PAGE_SIZE)
    const lastPage = Math.floor(lastVisible / PAGE_SIZE)

    // Fetch any pages we haven't fetched yet
    for (let page = firstPage; page <= lastPage; page++) {
      const offset = page * PAGE_SIZE
      if (!fetchedRanges.current.has(offset)) {
        fetchedRanges.current.add(offset)
        // Prefetch this page
        queryClient.prefetchQuery({
          queryKey: ['data-preview', offset],
          queryFn: () => getDataPreview({ offset, limit: PAGE_SIZE }),
        })
      }
    }
  }, [rowVirtualizer, queryClient])

  // Attach scroll handler
  useEffect(() => {
    const scrollElement = parentRef.current
    if (!scrollElement) return

    scrollElement.addEventListener('scroll', handleScroll)
    return () => scrollElement.removeEventListener('scroll', handleScroll)
  }, [handleScroll])

  if (isLoading && allRows.length === 0) {
    return (
      <div className="flex h-full items-center justify-center text-muted-foreground">
        Loading data...
      </div>
    )
  }

  if (error && allRows.length === 0) {
    return (
      <div className="flex h-full items-center justify-center text-destructive">
        Failed to load data
      </div>
    )
  }

  const highlightedRowSet = new Set(highlightedRows)
  const highlightedColIndex = highlightedColumn
    ? headers.indexOf(highlightedColumn)
    : -1

  const virtualItems = rowVirtualizer.getVirtualItems()

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between border-b bg-muted/50 px-3 py-2">
        <span className="text-sm font-medium">Data Preview</span>
        <span className="text-xs text-muted-foreground">
          {totalRows > PAGE_SIZE
            ? `Showing ${allRows.filter(r => r.length > 0).length} of ${totalRows} rows (scroll for more)`
            : `${totalRows} rows`}
        </span>
      </div>

      {/* Sticky header */}
      <div className="border-b bg-muted">
        <table className="w-full text-sm">
          <thead>
            <tr>
              <th className="w-12 border-r px-2 py-1.5 text-left text-xs font-medium text-muted-foreground">
                #
              </th>
              {headers.map((header, i) => (
                <th
                  key={header}
                  className={cn(
                    'border-r px-2 py-1.5 text-left text-xs font-medium',
                    i === highlightedColIndex && 'bg-primary/10 text-primary'
                  )}
                >
                  {header}
                </th>
              ))}
            </tr>
          </thead>
        </table>
      </div>

      {/* Virtualized body */}
      <div
        ref={parentRef}
        className="flex-1 overflow-auto"
        style={{ contain: 'strict' }}
      >
        <div
          style={{
            height: `${rowVirtualizer.getTotalSize()}px`,
            width: '100%',
            position: 'relative',
          }}
        >
          <table className="w-full text-sm" style={{ tableLayout: 'fixed' }}>
            <tbody>
              {virtualItems.map((virtualRow) => {
                const rowIndex = virtualRow.index
                const row = allRows[rowIndex] || []
                const isHighlighted = highlightedRowSet.has(rowIndex)
                const isLoaded = row.length > 0

                return (
                  <tr
                    key={virtualRow.key}
                    style={{
                      position: 'absolute',
                      top: 0,
                      left: 0,
                      width: '100%',
                      height: `${virtualRow.size}px`,
                      transform: `translateY(${virtualRow.start}px)`,
                    }}
                    className={cn(
                      'hover:bg-muted/50',
                      isHighlighted && 'bg-warning/20'
                    )}
                  >
                    <td className="w-12 border-b border-r px-2 py-1 text-xs text-muted-foreground">
                      {rowIndex + 1}
                    </td>
                    {isLoaded ? (
                      row.map((cell, colIndex) => (
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
                      ))
                    ) : (
                      // Loading placeholder
                      headers.map((_, colIndex) => (
                        <td
                          key={colIndex}
                          className="border-b border-r px-2 py-1 font-mono text-xs text-muted-foreground/50"
                        >
                          ...
                        </td>
                      ))
                    )}
                  </tr>
                )
              })}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  )
}
