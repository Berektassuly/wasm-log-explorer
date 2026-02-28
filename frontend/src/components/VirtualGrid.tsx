/**
 * Performance-optimized virtualized grid using react-virtuoso.
 * Row components are memoized to prevent layout thrashing; only visible slice is requested from the worker.
 */

import { memo, useCallback, useState, useRef } from 'react';
import { Virtuoso } from 'react-virtuoso';

type VirtualGridProps = {
  lineCount: number;
  getRows: (start: number, end: number) => Promise<string[]>;
};

const ROW_HEIGHT_PX = 22;
const OVERSCAN = 20;
const BATCH_SIZE = 150;

const Row = memo(function Row({ index, lineNumber, text }: { index: number; lineNumber: number; text: string }) {
  return (
    <div
      className="flex items-center border-b border-zinc-800 px-3 font-mono text-sm text-zinc-300 leading-tight"
      style={{ height: ROW_HEIGHT_PX, minHeight: ROW_HEIGHT_PX }}
      data-row-index={index}
    >
      <span className="select-none shrink-0 w-20 text-zinc-500 tabular-nums">{lineNumber}</span>
      <span className="truncate pl-2">{text || '\u00A0'}</span>
    </div>
  );
});

export const VirtualGrid = memo(function VirtualGrid({ lineCount, getRows }: VirtualGridProps) {
  const [slice, setSlice] = useState<Map<number, string>>(new Map());
  const loadingRef = useRef(false);
  const requestedRef = useRef<Set<string>>(new Set());

  const loadRange = useCallback(
    async (start: number, end: number) => {
      const endClamped = Math.min(end, lineCount);
      if (start >= endClamped) return;
      const key = `${start}-${endClamped}`;
      if (requestedRef.current.has(key)) return;
      requestedRef.current.add(key);
      loadingRef.current = true;
      try {
        const rows = await getRows(start, endClamped);
        setSlice((prev) => {
          const next = new Map(prev);
          rows.forEach((text, i) => next.set(start + i, text));
          return next;
        });
      } finally {
        requestedRef.current.delete(key);
        loadingRef.current = false;
      }
    },
    [getRows, lineCount]
  );

  const itemContent = useCallback(
    (index: number) => {
      const lineNumber = index + 1;
      const text = slice.get(index);
      if (text === undefined && !loadingRef.current) {
        const start = Math.max(0, index - OVERSCAN);
        const end = Math.min(lineCount, index + BATCH_SIZE);
        loadRange(start, end);
      }
      return <Row index={index} lineNumber={lineNumber} text={text ?? ''} />;
    },
    [slice, loadRange, lineCount]
  );

  return (
    <div className="h-full w-full flex flex-col bg-zinc-950 text-zinc-300 font-mono">
      <div className="shrink-0 border-b border-zinc-800 px-3 py-2 text-zinc-500 text-sm">
        {lineCount.toLocaleString()} lines
      </div>
      <div className="flex-1 min-h-0">
        <Virtuoso
          style={{ height: '100%' }}
          totalCount={lineCount}
          itemContent={itemContent}
          overscan={OVERSCAN}
          defaultItemHeight={ROW_HEIGHT_PX}
          increaseViewportBy={{ top: 200, bottom: 200 }}
        />
      </div>
    </div>
  );
});
