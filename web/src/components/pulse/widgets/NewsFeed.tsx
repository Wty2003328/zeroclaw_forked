import { useState } from 'react';
import { useWidgetData } from '../../../hooks/useWidgetData';
import { timeAgo } from '../../../lib/time';
import { Badge } from '../ui/badge';
import { ChevronDown, ChevronUp, ExternalLink } from 'lucide-react';
import type { WidgetDimensions } from '../../../lib/widget-size';
import type { FeedResponse, FeedItem } from '../../../types/pulse';

function sourceLabel(source: string): string {
  let label = source;
  if (label.startsWith('rss:')) label = label.slice(4);
  return label.replace(/[-_]/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase());
}

function stripHtml(html: string): string {
  const doc = new DOMParser().parseFromString(html, 'text/html');
  return doc.body.textContent || '';
}

function FeedItemRow({ item }: { item: FeedItem }) {
  const [expanded, setExpanded] = useState(false);
  const hasContent = !!(item.content || item.summary || item.url);

  return (
    <div className="rounded-lg transition-colors hover:bg-accent/40">
      <div className="px-2.5 py-1.5 cursor-pointer" onClick={() => hasContent && setExpanded(!expanded)}>
        <div className="flex items-center gap-2 mb-0.5">
          <span className="cq-text-sm font-semibold text-primary bg-primary/10 px-1.5 py-0.5 rounded">{sourceLabel(item.source)}</span>
          <span className="cq-text-xs text-muted-foreground">{item.published_at ? timeAgo(item.published_at) : timeAgo(item.collected_at)}</span>
          {hasContent && <span className="ml-auto">{expanded ? <ChevronUp className="w-3 h-3 text-muted-foreground"/> : <ChevronDown className="w-3 h-3 text-muted-foreground"/>}</span>}
        </div>
        <h3 className="cq-text-base font-medium leading-snug pl-2">{item.title}</h3>
      </div>
      {expanded && (
        <div className="pl-5 pr-2.5 pb-2 border-t border-border/40 pt-1.5 space-y-1.5">
          {item.summary && <p className="text-sm text-muted-foreground leading-relaxed italic">{item.summary}</p>}
          {item.content && <div className="text-sm text-foreground/80 leading-relaxed max-h-48 overflow-y-auto">{stripHtml(item.content).slice(0, 1000)}{stripHtml(item.content).length > 1000 && '...'}</div>}
          {item.url && <a href={item.url} target="_blank" rel="noopener noreferrer" className="inline-flex items-center gap-1 text-xs text-primary hover:underline" onClick={(e) => e.stopPropagation()}>Open article <ExternalLink className="w-3 h-3"/></a>}
        </div>
      )}
    </div>
  );
}

interface Props { dims?: WidgetDimensions }

export default function NewsFeed({ }: Props) {
  const { data, loading, error } = useWidgetData<FeedResponse>('/api/pulse/feed?limit=50', 60000);

  if (loading) return <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm">Loading...</div>;
  if (error) return <div className="flex-1 flex items-center justify-center text-destructive text-sm">Error</div>;
  if (!data || data.items.length === 0) return <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm">No items</div>;

  return (
    <div className="flex flex-col overflow-hidden h-full">
      {/* Tiny: just titles */}
      <div className="@[200px]:hidden flex-1 overflow-y-auto">
        {data.items.map((item) => (
          <div key={item.id} className="py-0.5 truncate cq-text-xs text-foreground cursor-pointer hover:text-primary"
            onClick={() => item.url && window.open(item.url, '_blank')}>
            {item.title}
          </div>
        ))}
      </div>
      {/* Normal: full rows */}
      <div className="hidden @[200px]:flex flex-col overflow-hidden h-full">
        <div className="flex items-center mb-1 shrink-0">
          <Badge variant="secondary" className="text-xs px-1.5 py-0">{data.count}</Badge>
        </div>
        <div className="flex-1 overflow-y-auto">
          {data.items.map((item) => <FeedItemRow key={item.id} item={item} />)}
        </div>
      </div>
    </div>
  );
}
