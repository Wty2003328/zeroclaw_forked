import { useWidgetData } from '../../../hooks/useWidgetData';
import { timeAgo } from '../../../lib/time';
import { Play } from 'lucide-react';
import type { WidgetDimensions } from '../../../lib/widget-size';
import type { FeedResponse, FeedItem } from '../../../types/pulse';

function platformLabel(source: string): string {
  if (source.includes('youtube')) return 'YT';
  if (source.includes('bilibili')) return 'B站';
  return 'Vid';
}

function platformColor(source: string): string {
  if (source.includes('youtube')) return 'text-red-400 bg-red-400/10';
  if (source.includes('bilibili')) return 'text-pink-400 bg-pink-400/10';
  return 'text-primary bg-primary/10';
}

function VideoRow({ item }: { item: FeedItem }) {
  const m = item.metadata as Record<string, unknown>;
  const thumbnail = m.thumbnail as string | undefined;
  const channelName = (m.channel_name || m.author || '') as string;

  return (
    <a href={item.url ?? '#'} target="_blank" rel="noopener noreferrer"
      className="flex gap-2 p-1.5 rounded-lg hover:bg-accent/40 transition-colors no-underline group">
      {/* Thumbnail — at wider containers */}
      {thumbnail && (
        <div className="hidden @[250px]:block w-20 h-12 rounded overflow-hidden shrink-0 bg-muted relative">
          <img src={thumbnail} alt="" className="w-full h-full object-cover" loading="lazy" />
          <Play className="absolute inset-0 m-auto w-5 h-5 text-white/80 opacity-0 group-hover:opacity-100 transition-opacity" />
        </div>
      )}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-1.5 mb-0.5">
          <span className={`cq-text-xs font-semibold px-1 py-0.5 rounded ${platformColor(item.source)}`}>
            {platformLabel(item.source)}
          </span>
          <span className="cq-text-xs text-muted-foreground truncate">{channelName}</span>
          <span className="cq-text-xs text-muted-foreground ml-auto shrink-0">
            {item.published_at ? timeAgo(item.published_at) : ''}
          </span>
        </div>
        <h3 className="cq-text-sm font-medium leading-snug line-clamp-2 group-hover:text-primary transition-colors">
          {item.title}
        </h3>
      </div>
    </a>
  );
}

interface Props { dims?: WidgetDimensions }

export default function Videos({ }: Props) {
  const { data, loading, error } = useWidgetData<FeedResponse>('/api/pulse/feed?source=video&limit=30', 120000);

  if (loading) return <div className="flex-1 flex items-center justify-center text-muted-foreground cq-text-sm">Loading...</div>;
  if (error) return <div className="flex-1 flex items-center justify-center text-destructive cq-text-sm">Error</div>;
  if (!data || data.items.length === 0) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center gap-1 text-center">
        <Play className="w-6 h-6 text-muted-foreground/40" />
        <p className="hidden @[130px]:block cq-text-xs text-muted-foreground">Add channels in Settings</p>
      </div>
    );
  }

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
        <div className="flex-1 overflow-y-auto">
          {data.items.map((item) => <VideoRow key={item.id} item={item} />)}
        </div>
      </div>
    </div>
  );
}
