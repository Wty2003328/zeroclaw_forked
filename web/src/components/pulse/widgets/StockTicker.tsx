import { useState } from 'react';
import { useWidgetData } from '../../../hooks/useWidgetData';
import { cn } from '../../../lib/pulseUtils';
import { ChevronDown, ChevronUp, ExternalLink, TrendingUp, TrendingDown, Minus } from 'lucide-react';
import type { WidgetDimensions } from '../../../lib/widget-size';
import type { FeedResponse, FeedItem } from '../../../types/pulse';

interface StockMeta {
  symbol: string; name: string; price: number; change: number; change_percent: number; direction: string;
  volume?: number; previous_close?: number; open?: number; day_high?: number; day_low?: number;
  '52w_high'?: number; '52w_low'?: number; market_cap?: number;
}

function fmt(n: number | undefined) { if(n==null)return'-';if(n>=1e12)return`${(n/1e12).toFixed(1)}T`;if(n>=1e9)return`${(n/1e9).toFixed(1)}B`;if(n>=1e6)return`${(n/1e6).toFixed(1)}M`;if(n>=1e3)return`${(n/1e3).toFixed(1)}K`;return n.toFixed(2); }

function DirIcon({ dir }: { dir: string }) {
  if (dir === 'up') return <TrendingUp className="w-3.5 h-3.5 text-success shrink-0"/>;
  if (dir === 'down') return <TrendingDown className="w-3.5 h-3.5 text-destructive shrink-0"/>;
  return <Minus className="w-3.5 h-3.5 text-muted-foreground shrink-0"/>;
}

function StockRow({ item }: { item: FeedItem }) {
  const [expanded, setExpanded] = useState(false);
  const m = item.metadata as unknown as StockMeta;
  const pos = m.direction === 'up'; const neg = m.direction === 'down';
  const color = pos ? 'text-success' : neg ? 'text-destructive' : 'text-muted-foreground';
  const bg = pos ? 'bg-success/8' : neg ? 'bg-destructive/8' : 'bg-muted';

  return (
    <div className={cn('rounded-md overflow-hidden', bg)}>
      <div className="flex items-center gap-1 px-1.5 py-1 cursor-pointer overflow-hidden" onClick={() => setExpanded(!expanded)}>
        <DirIcon dir={m.direction} />
        <span className="cq-text-sm font-bold uppercase shrink-0">{m.symbol}</span>
        <span className={cn('cq-text-sm font-semibold shrink-0 ml-auto', color)}>${m.price.toFixed(2)}</span>
        <span className={cn('hidden @[170px]:inline cq-text-xs font-medium px-1 py-0.5 rounded shrink-0', pos && 'bg-success/15', neg && 'bg-destructive/15', color)}>
          {pos?'+':''}{m.change_percent.toFixed(1)}%
        </span>
        <span className="hidden @[120px]:inline shrink-0">{expanded ? <ChevronUp className="w-3 h-3 text-muted-foreground"/> : <ChevronDown className="w-3 h-3 text-muted-foreground"/>}</span>
      </div>
      {expanded && (
        <div className="px-2 pb-1.5 border-t border-border/30 pt-1.5">
          <div className="grid grid-cols-2 gap-x-3 gap-y-1 text-xs">
            <div className="flex justify-between"><span className="text-muted-foreground">Open</span><span className="font-medium">${m.open?.toFixed(2) ?? '-'}</span></div>
            <div className="flex justify-between"><span className="text-muted-foreground">Prev</span><span className="font-medium">${m.previous_close?.toFixed(2) ?? '-'}</span></div>
            <div className="flex justify-between"><span className="text-muted-foreground">High</span><span className="font-medium text-destructive/70">${m.day_high?.toFixed(2) ?? '-'}</span></div>
            <div className="flex justify-between"><span className="text-muted-foreground">Low</span><span className="font-medium text-blue-400/70">${m.day_low?.toFixed(2) ?? '-'}</span></div>
            <div className="flex justify-between"><span className="text-muted-foreground">52W H</span><span className="font-medium">${m['52w_high']?.toFixed(2) ?? '-'}</span></div>
            <div className="flex justify-between"><span className="text-muted-foreground">52W L</span><span className="font-medium">${m['52w_low']?.toFixed(2) ?? '-'}</span></div>
            <div className="flex justify-between"><span className="text-muted-foreground">Vol</span><span className="font-medium">{fmt(m.volume)}</span></div>
            <div className="flex justify-between"><span className="text-muted-foreground">Cap</span><span className="font-medium">{fmt(m.market_cap)}</span></div>
          </div>
          {item.url && <a href={item.url} target="_blank" rel="noopener noreferrer" className="inline-flex items-center gap-0.5 text-xs text-primary hover:underline mt-1" onClick={(e) => e.stopPropagation()}>Yahoo <ExternalLink className="w-2.5 h-2.5"/></a>}
        </div>
      )}
    </div>
  );
}

interface Props { dims?: WidgetDimensions }

export default function StockTicker({ }: Props) {
  const { data, loading, error } = useWidgetData<FeedResponse>('/api/pulse/feed?source=stock&limit=20', 60000);

  if (loading) return <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm">Loading...</div>;
  if (error) return <div className="flex-1 flex items-center justify-center text-destructive text-sm">Error</div>;
  if (!data || data.items.length === 0) return <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm">No stock data</div>;

  return (
    <div className="flex flex-col overflow-hidden h-full">
      {/* Tiny: plain text symbol + price */}
      <div className="@[200px]:hidden flex-1 overflow-y-auto flex flex-col justify-evenly">
        {data.items.map((i) => {
          const sm = i.metadata as unknown as StockMeta;
          const c = sm.direction === 'up' ? 'text-success' : sm.direction === 'down' ? 'text-destructive' : 'text-muted-foreground';
          return (
            <div key={i.id} className="flex items-center justify-between cq-text-xs">
              <span className="font-bold">{sm.symbol}</span>
              <span className={cn('font-semibold', c)}>${sm.price.toFixed(0)}</span>
            </div>
          );
        })}
      </div>
      {/* Normal: full rows */}
      <div className="hidden @[200px]:flex flex-col overflow-hidden h-full">
        <div className="flex-1 overflow-y-auto flex flex-col justify-evenly">
          {data.items.map((i) => <StockRow key={i.id} item={i} />)}
        </div>
      </div>
    </div>
  );
}
