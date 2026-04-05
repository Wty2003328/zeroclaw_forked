import { useWidgetData } from '../../../hooks/useWidgetData';
import { Card, CardContent } from '../ui/card';

export function AboutPanel() {
  const { data } = useWidgetData<{ status: string; version: string }>('/api/health', 60000);

  return (
    <div>
      <h2 className="text-lg font-semibold mb-1">About</h2>
      <p className="text-sm text-muted-foreground mb-6">System information.</p>

      <Card>
        <CardContent className="p-4 space-y-3">
          <Row label="Application" value="Pulse — Personal Intelligence Dashboard" />
          <Row label="Version" value={data?.version ?? '...'} />
          <Row label="Backend" value="Rust (Axum + Tokio + SQLite)" />
          <Row label="Frontend" value="React 19 + TypeScript + Tailwind CSS" />
          <Row label="License" value="MIT" />
          <Row
            label="Repository"
            value={
              <a
                href="https://github.com/Wty2003328/pulse"
                target="_blank"
                rel="noopener noreferrer"
                className="text-primary hover:underline"
              >
                github.com/Wty2003328/pulse
              </a>
            }
          />
        </CardContent>
      </Card>
    </div>
  );
}

function Row({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between py-1.5 border-b border-border last:border-0">
      <span className="text-sm text-muted-foreground">{label}</span>
      <span className="text-sm font-medium text-foreground">{value}</span>
    </div>
  );
}
