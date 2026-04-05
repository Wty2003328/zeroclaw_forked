import { useWidgetData } from '../../../hooks/useWidgetData';
import { Badge } from '../ui/badge';
import { Button } from '../ui/button';
import type { WidgetDimensions } from '../../../lib/widget-size';
import type { CollectorsResponse } from '../../../types/pulse';

interface Props { dims?: WidgetDimensions }

export default function CollectorStatus({ }: Props) {
  const { data, loading, error, refetch } = useWidgetData<CollectorsResponse>('/api/pulse/collectors', 30000);

  const triggerCollector = async (id: string) => {
    try {
      await fetch(`/api/collectors/${id}/run`, { method: 'POST' });
      setTimeout(() => {
        refetch();
        // Notify all widgets to refresh their data
        window.dispatchEvent(new CustomEvent('pulse-data-refresh'));
      }, 2000);
    } catch (err) { console.error('Failed to trigger collector:', err); }
  };

  if (loading) return <div className="flex-1 flex items-center justify-center text-muted-foreground cq-text-sm">Loading...</div>;
  if (error) return <div className="flex-1 flex items-center justify-center text-destructive cq-text-sm">Error</div>;
  if (!data) return null;

  return (
    <div className="flex flex-col overflow-hidden h-full">
      {/* Tiny: simple dot + name list */}
      <div className="@[200px]:hidden flex-1 overflow-y-auto flex flex-col justify-evenly">
        {data.collectors.map((c) => {
          const ok = c.enabled && data.recent_runs.find(r => r.collector_id === c.id)?.status !== 'error';
          return (
            <div key={c.id} className="flex items-center gap-1.5 py-0.5">
              <div className={`w-2 h-2 rounded-full shrink-0 ${ok ? 'bg-success' : c.enabled ? 'bg-destructive' : 'bg-muted-foreground/40'}`} />
              <span className="cq-text-xs truncate">{c.name}</span>
            </div>
          );
        })}
      </div>

      {/* Normal: full cards */}
      <div className="hidden @[200px]:flex flex-col flex-1 overflow-y-auto gap-1.5">
        {data.collectors.map((collector) => {
          const lastRun = data.recent_runs.find((r) => r.collector_id === collector.id);
          return (
            <div key={collector.id} className="p-2 bg-muted rounded-lg">
              <div className="flex items-center justify-between mb-0.5">
                <span className="cq-text-sm font-medium truncate">{collector.name}</span>
                <Badge variant={collector.enabled ? 'success' : 'secondary'} className="text-xs px-1 py-0">{collector.enabled ? 'ON' : 'OFF'}</Badge>
              </div>
              {lastRun && (
                <div className="flex items-center gap-1.5 mb-1">
                  <Badge variant={lastRun.status === 'success' ? 'success' : lastRun.status === 'running' ? 'warning' : 'destructive'} className="text-xs px-1 py-0">{lastRun.status}</Badge>
                  <span className="cq-text-xs text-muted-foreground">{lastRun.items_count} items</span>
                </div>
              )}
              {collector.enabled && <Button variant="outline" size="sm" className="w-full h-6 text-xs" onClick={() => triggerCollector(collector.id)}>Run Now</Button>}
            </div>
          );
        })}
      </div>
    </div>
  );
}
