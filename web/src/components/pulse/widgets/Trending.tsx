import { useMemo } from 'react';
import { useWidgetData } from '../../../hooks/useWidgetData';
import type { WidgetDimensions } from '../../../lib/widget-size';
import type { FeedResponse } from '../../../types/pulse';

interface Props { dims?: WidgetDimensions }

export default function Trending({ }: Props) {
  const { data, loading, error } = useWidgetData<FeedResponse>('/api/pulse/feed?limit=100', 60000);

  const trendingTags = useMemo(() => {
    if (!data || !data.items) return [];
    const tagMap = new Map<string, number>();
    data.items.forEach((item) => { item.tags.forEach((tag) => tagMap.set(tag, (tagMap.get(tag) || 0) + 1)); });
    return Array.from(tagMap.entries()).map(([tag, count]) => ({ tag, count })).sort((a, b) => b.count - a.count).slice(0, 30);
  }, [data]);

  if (loading) return <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm">Loading...</div>;
  if (error) return <div className="flex-1 flex items-center justify-center text-destructive text-sm">Error</div>;
  if (trendingTags.length === 0) return <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm">No tags yet</div>;

  const maxCount = Math.max(...trendingTags.map((t) => t.count));

  return (
    <div className="flex flex-col overflow-hidden h-full">
      <div className="flex-1 overflow-y-auto flex flex-wrap gap-1.5 content-start">
        {trendingTags.map((item) => {
          const intensity = Math.min(100, (item.count / maxCount) * 100);
          return (
            <div key={item.tag}
              className="inline-flex items-center gap-0.5 px-1.5 py-1 bg-primary/8 border border-primary/20 rounded-md text-primary font-medium hover:bg-primary/15 transition-colors cursor-default text-xs"
              style={{ opacity: 0.6 + (intensity / 100) * 0.4 }}
              title={`${item.count} items`}>
              {item.tag}
              <span className="hidden @[200px]:inline text-[0.55rem] text-muted-foreground bg-muted px-0.5 rounded">{item.count}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
