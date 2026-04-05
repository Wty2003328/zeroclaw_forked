import { useState, useEffect } from 'react';
import { useWidgetData } from '../../../hooks/useWidgetData';
import { Calendar as CalIcon, MapPin } from 'lucide-react';
import { Button } from '../ui/button';
import type { WidgetDimensions } from '../../../lib/widget-size';

interface CalendarEvent { id: string; title: string; start: string; end: string; all_day: boolean; location: string }
interface CalendarResponse { events: CalendarEvent[] }

function formatTime(iso: string) { try { return new Date(iso).toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit', hour12: true }); } catch { return iso; } }
function formatDate(iso: string) { try { const d=new Date(iso),t=new Date();t.setHours(0,0,0,0);const diff=(d.getTime()-t.getTime())/864e5;if(diff>=0&&diff<1)return'Today';if(diff>=1&&diff<2)return'Tomorrow';return d.toLocaleDateString('en-US',{weekday:'short',month:'short',day:'numeric'});} catch{return'';} }
function isNow(s: string, e: string) { const now=Date.now();return new Date(s).getTime()<=now&&now<=new Date(e).getTime(); }

interface Props { dims?: WidgetDimensions }

export default function Calendar({ }: Props) {
  const [connected, setConnected] = useState<boolean | null>(null);
  const [hasCredentials, setHasCredentials] = useState(false);
  const [connecting, setConnecting] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    // Check calendar status
    fetch('/api/pulse/calendar/status').then(r => r.json())
      .then(s => setConnected(s.connected))
      .catch(() => setConnected(false));
    // Check if credentials are configured
    fetch('/api/pulse/settings/app').then(r => r.json())
      .then(d => setHasCredentials(!!(d.google_client_id && d.google_client_secret)))
      .catch(() => {});
  }, []);

  const handleConnect = async () => {
    setConnecting(true);
    setError('');
    try {
      const res = await fetch('/api/pulse/calendar/auth-url');
      if (!res.ok) {
        const errText = await res.text();
        setError(errText);
        setConnecting(false);
        return;
      }
      const { url } = await res.json();
      const popup = window.open(url, 'google-auth', 'width=500,height=600');
      const poll = setInterval(async () => {
        const s = await fetch('/api/pulse/calendar/status').then(r => r.json()).catch(() => ({ connected: false }));
        if (s.connected) { clearInterval(poll); setConnected(true); setConnecting(false); if (popup) popup.close(); }
      }, 2000);
      setTimeout(() => { clearInterval(poll); setConnecting(false); }, 120000);
    } catch { setConnecting(false); setError('Connection failed'); }
  };

  // Loading
  if (connected === null) return <div className="flex-1 flex items-center justify-center text-muted-foreground cq-text-sm">Loading...</div>;

  // Not connected
  if (!connected) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center gap-2 text-center p-2">
        <CalIcon className="w-8 h-8 text-muted-foreground/40" />
        {!hasCredentials ? (
          <>
            <p className="hidden @[130px]:block cq-text-xs text-muted-foreground">Set up Google Calendar credentials in Settings &gt; Data Sources first</p>
          </>
        ) : (
          <>
            <p className="hidden @[130px]:block cq-text-xs text-muted-foreground">Connect your Google Calendar</p>
            <div className="hidden @[150px]:block">
              <Button size="sm" onClick={handleConnect} disabled={connecting}>
                {connecting ? 'Connecting...' : 'Connect'}
              </Button>
            </div>
            {/* Tiny size: click anywhere */}
            <div className="@[150px]:hidden w-full h-full absolute inset-0 cursor-pointer" onClick={handleConnect} />
          </>
        )}
        {error && <p className="cq-text-xs text-destructive">{error}</p>}
      </div>
    );
  }

  return <CalendarEvents />;
}

function CalendarEvents() {
  const { data, loading, error } = useWidgetData<CalendarResponse>('/api/pulse/calendar/events', 300000);

  if (loading) return <div className="flex-1 flex items-center justify-center text-muted-foreground cq-text-sm">Loading events...</div>;
  if (error) return <div className="flex-1 flex items-center justify-center text-destructive cq-text-sm">Error loading calendar</div>;
  if (!data || data.events.length === 0) return <div className="flex-1 flex items-center justify-center text-muted-foreground cq-text-sm">No upcoming events</div>;

  const grouped = new Map<string, CalendarEvent[]>();
  for (const ev of data.events) { const day = formatDate(ev.start||ev.end); if (!grouped.has(day)) grouped.set(day,[]); grouped.get(day)!.push(ev); }

  return (
    <div className="flex flex-col h-full overflow-y-auto gap-2">
      {Array.from(grouped.entries()).map(([day, events]) => (
        <div key={day}>
          <div className="hidden @[130px]:block text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-1">{day}</div>
          <div className="flex flex-col gap-0.5">
            {events.map((ev) => {
              const current = isNow(ev.start, ev.end);
              return (
                <div key={ev.id} className={`flex gap-2 p-1.5 rounded-md border-l-2 ${current ? 'border-l-primary bg-primary/5' : 'border-l-border'}`}>
                  <div className="hidden @[200px]:block w-14 shrink-0 text-right">
                    {ev.all_day ? <span className="cq-text-xs text-muted-foreground">All day</span> : (
                      <div><div className="cq-text-sm font-medium">{formatTime(ev.start)}</div><div className="cq-text-xs text-muted-foreground">{formatTime(ev.end)}</div></div>
                    )}
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="cq-text-base font-medium truncate">{ev.title}</div>
                    {ev.location && <div className="hidden @[300px]:flex text-xs text-muted-foreground items-center gap-0.5 truncate"><MapPin className="w-2.5 h-2.5 shrink-0"/>{ev.location}</div>}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      ))}
    </div>
  );
}
