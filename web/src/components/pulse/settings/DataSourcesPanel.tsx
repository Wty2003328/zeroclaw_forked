import { useState, useEffect } from 'react';
import { useWidgetData } from '../../../hooks/useWidgetData';
import { Card, CardContent } from '../ui/card';
import { Badge } from '../ui/badge';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Loader2, Pencil, Check, X, Plus, Trash2, Rss, MapPin, TrendingUp, Calendar, Bot, ExternalLink, CheckCircle2, XCircle, Play } from 'lucide-react';
import type { CollectorsResponse } from '../../../types/pulse';

interface UserFeed { name: string; url: string }
interface Props { onToast: (message: string, type: 'success' | 'error') => void }

export function DataSourcesPanel({ onToast }: Props) {
  const { data, loading, refetch } = useWidgetData<CollectorsResponse>('/api/pulse/collectors', 30000);
  const [feeds, setFeeds] = useState<UserFeed[]>([]);
  const [showAddFeed, setShowAddFeed] = useState(false);
  const [weatherLocation, setWeatherLocation] = useState('');
  const [stockSymbols, setStockSymbols] = useState('');
  const [rsshubUrl, setRsshubUrl] = useState('');
  const [googleClientId, setGoogleClientId] = useState('');
  const [googleClientSecret, setGoogleClientSecret] = useState('');
  const [calendarConnected, setCalendarConnected] = useState(false);
  const [calendarConnecting, setCalendarConnecting] = useState(false);
  const [videoChannels, setVideoChannels] = useState<{ platform: string; channel_id: string; name: string }[]>([]);
  const [showAddVideo, setShowAddVideo] = useState(false);
  const [zeroclawUrl, setZeroclawUrl] = useState('');
  const [zeroclawToken, setZeroclawToken] = useState('');
  const [zeroclawStatus, setZeroclawStatus] = useState<{ configured: boolean; reachable: boolean } | null>(null);
  const [, setSettingsLoaded] = useState(false);

  const fetchFeeds = async () => {
    try { const res = await fetch('/api/pulse/settings/feeds'); if (res.ok) { const d = await res.json(); setFeeds(d.feeds || []); } } catch {}
  };

  const fetchAppSettings = async () => {
    try {
      const res = await fetch('/api/pulse/settings/app');
      if (res.ok) {
        const d = await res.json();
        if (d.weather_location) setWeatherLocation(d.weather_location);
        if (d.stock_symbols) setStockSymbols(d.stock_symbols);
        if (d.rsshub_url) setRsshubUrl(d.rsshub_url);
        if (d.google_client_id) setGoogleClientId(d.google_client_id);
        if (d.google_client_secret) setGoogleClientSecret(d.google_client_secret);
      }
    } catch {}
    // Calendar status
    try { const r = await fetch('/api/pulse/calendar/status'); if (r.ok) { const s = await r.json(); setCalendarConnected(s.connected); } } catch {}
    // ZeroClaw config + status
    try {
      const r = await fetch('/api/pulse/zeroclaw/config'); if (r.ok) { const c = await r.json(); if (c.url) setZeroclawUrl(c.url); if (c.token) setZeroclawToken(c.token); }
      const s = await fetch('/api/pulse/zeroclaw/status'); if (s.ok) setZeroclawStatus(await s.json());
    } catch {}
    setSettingsLoaded(true);
  };

  const saveRsshubUrl = async () => {
    try {
      const res = await fetch('/api/pulse/settings/app', { method: 'PUT', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ rsshub_url: rsshubUrl }) });
      if (res.ok) onToast(rsshubUrl ? `RSSHub URL set to ${rsshubUrl}` : 'RSSHub URL cleared', 'success');
    } catch { onToast('Failed', 'error'); }
  };

  const fetchVideoChannels = async () => {
    try { const r = await fetch('/api/pulse/settings/videos'); if (r.ok) { const d = await r.json(); setVideoChannels(d.channels || []); } } catch {}
  };

  useEffect(() => { fetchFeeds(); fetchAppSettings(); fetchVideoChannels(); }, []);

  const triggerCollector = async (id: string, name: string) => {
    try { await fetch(`/api/collectors/${id}/run`, { method: 'POST' }); onToast(`${name} triggered`, 'success'); setTimeout(refetch, 2000); }
    catch { onToast('Failed', 'error'); }
  };

  const saveInterval = async (id: string, name: string, secs: number) => {
    try {
      const res = await fetch(`/api/settings/collectors/${id}/interval`, { method: 'PUT', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ interval_secs: secs }) });
      if (res.ok) { onToast(`${name} interval updated`, 'success'); refetch(); } else { onToast('Failed', 'error'); }
    } catch { onToast('Failed', 'error'); }
  };

  const addFeed = async (name: string, url: string) => {
    try {
      const res = await fetch('/api/pulse/settings/feeds', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ name, url }) });
      if (res.ok) { onToast(`Feed "${name}" added`, 'success'); fetchFeeds(); setShowAddFeed(false); } else { onToast('Failed', 'error'); }
    } catch { onToast('Failed', 'error'); }
  };

  const removeFeed = async (url: string, name: string) => {
    try {
      const res = await fetch(`/api/settings/feeds/${encodeURIComponent(url)}`, { method: 'DELETE' });
      if (res.ok) { onToast(`"${name}" removed`, 'success'); fetchFeeds(); }
    } catch { onToast('Failed', 'error'); }
  };

  const saveWeatherLocation = async () => {
    try {
      const res = await fetch('/api/pulse/settings/app', { method: 'PUT', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ weather_location: weatherLocation }) });
      if (res.ok) onToast(`Weather location set to "${weatherLocation}"`, 'success'); else onToast('Failed', 'error');
    } catch { onToast('Failed', 'error'); }
  };

  const saveStockSymbols = async () => {
    try {
      const res = await fetch('/api/pulse/settings/app', { method: 'PUT', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ stock_symbols: stockSymbols }) });
      if (res.ok) onToast('Stock symbols saved', 'success'); else onToast('Failed', 'error');
    } catch { onToast('Failed', 'error'); }
  };

  return (
    <div>
      <h2 className="text-lg font-semibold mb-1">Data Sources</h2>
      <p className="text-sm text-muted-foreground mb-6">Configure what data feeds your dashboard.</p>

      {/* Weather Location */}
      <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-3 flex items-center gap-1.5"><MapPin className="w-3.5 h-3.5" />Weather</h3>
      <Card className="mb-6">
        <CardContent className="p-4">
          <p className="text-xs text-muted-foreground mb-2">Weather data from <a href="https://wttr.in" target="_blank" rel="noopener noreferrer" className="text-primary hover:underline">wttr.in</a> (free, no API key needed). Enter a city name.</p>
          <div className="flex gap-2">
            <Input value={weatherLocation} onChange={(e) => setWeatherLocation(e.target.value)} placeholder="e.g. San Francisco, London, Tokyo"
              onKeyDown={(e) => { if (e.key === 'Enter') saveWeatherLocation(); }} className="flex-1" />
            <Button variant="outline" size="sm" onClick={saveWeatherLocation} disabled={!weatherLocation.trim()}>Save</Button>
          </div>
        </CardContent>
      </Card>

      {/* Stock Symbols */}
      <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-3 flex items-center gap-1.5"><TrendingUp className="w-3.5 h-3.5" />Stocks</h3>
      <Card className="mb-6">
        <CardContent className="p-4">
          <p className="text-xs text-muted-foreground mb-2">Stock data from Yahoo Finance (free, no API key). Enter comma-separated ticker symbols.</p>
          <div className="flex gap-2">
            <Input value={stockSymbols} onChange={(e) => setStockSymbols(e.target.value)} placeholder="e.g. AAPL, GOOGL, MSFT, NVDA, TSLA"
              onKeyDown={(e) => { if (e.key === 'Enter') saveStockSymbols(); }} className="flex-1" />
            <Button variant="outline" size="sm" onClick={saveStockSymbols} disabled={!stockSymbols.trim()}>Save</Button>
          </div>
          {stockSymbols && (
            <div className="flex flex-wrap gap-1 mt-2">
              {stockSymbols.split(',').map(s => s.trim()).filter(Boolean).map(s => (
                <span key={s} className="text-[0.65rem] font-bold text-foreground bg-muted px-1.5 py-0.5 rounded">{s}</span>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Video Subscriptions */}
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider flex items-center gap-1.5"><Play className="w-3.5 h-3.5" />Video Subscriptions</h3>
        {!showAddVideo && <Button variant="outline" size="sm" onClick={() => setShowAddVideo(true)}><Plus className="w-3.5 h-3.5 mr-1"/>Add Channel</Button>}
      </div>
      <Card className="mb-6">
        <CardContent className="p-4">
          <p className="text-xs text-muted-foreground mb-3">
            Follow YouTube channels and Bilibili UP主. YouTube uses built-in RSS. Bilibili requires a self-hosted <a href="https://docs.rsshub.app/deploy/" target="_blank" rel="noopener noreferrer" className="text-primary hover:underline">RSSHub</a> instance.
          </p>

          <div className="mb-3">
            <label className="text-xs font-medium text-muted-foreground mb-1 block">RSSHub URL (for Bilibili)</label>
            <div className="flex gap-2">
              <Input value={rsshubUrl} onChange={(e) => setRsshubUrl(e.target.value)}
                placeholder="http://localhost:1200 or your self-hosted URL"
                onKeyDown={(e) => { if (e.key === 'Enter') saveRsshubUrl(); }} className="flex-1" />
              <Button variant="outline" size="sm" onClick={saveRsshubUrl}>Save</Button>
            </div>
            <p className="text-xs text-muted-foreground mt-1">Run <code className="bg-muted px-1 rounded">docker compose --profile rsshub up -d</code> to start a local RSSHub, then enter <code className="bg-muted px-1 rounded">http://localhost:1200</code></p>
          </div>

          {showAddVideo && <AddVideoForm onAdd={async (platform, channelId, name) => {
            try {
              const res = await fetch('/api/pulse/settings/videos', { method: 'POST', headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ platform, channel_id: channelId, name }) });
              if (res.ok) { onToast(`Added ${name}`, 'success'); fetchVideoChannels(); setShowAddVideo(false); }
              else { onToast(`Failed: ${await res.text()}`, 'error'); }
            } catch { onToast('Failed', 'error'); }
          }} onCancel={() => setShowAddVideo(false)} />}

          {videoChannels.length > 0 ? (
            <div className="space-y-2 mt-2">
              {videoChannels.map((ch) => (
                <div key={`${ch.platform}-${ch.channel_id}`} className="flex items-center gap-3 p-2 bg-muted rounded-lg">
                  <span className={`text-xs font-bold px-1.5 py-0.5 rounded ${ch.platform === 'youtube' ? 'text-red-400 bg-red-400/10' : 'text-pink-400 bg-pink-400/10'}`}>
                    {ch.platform === 'youtube' ? 'YT' : 'B站'}
                  </span>
                  <div className="flex-1 min-w-0">
                    <div className="text-sm font-medium truncate">{ch.name}</div>
                    <div className="text-xs text-muted-foreground truncate">{ch.channel_id}</div>
                  </div>
                  <Button variant="ghost" size="sm" className="text-destructive hover:text-destructive hover:bg-destructive/10 shrink-0"
                    onClick={async () => {
                      await fetch(`/api/settings/videos/${ch.platform}/${ch.channel_id}`, { method: 'DELETE' });
                      onToast(`Removed ${ch.name}`, 'success'); fetchVideoChannels();
                    }}><Trash2 className="w-3.5 h-3.5"/></Button>
                </div>
              ))}
            </div>
          ) : !showAddVideo && (
            <div className="text-center text-muted-foreground text-sm py-2">No channels added yet.</div>
          )}
        </CardContent>
      </Card>

      {/* Google Calendar */}
      <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-3 flex items-center gap-1.5"><Calendar className="w-3.5 h-3.5" />Google Calendar</h3>
      <Card className="mb-6">
        <CardContent className="p-4 space-y-3">
          <div className="flex items-center justify-between mb-1">
            <p className="text-xs text-muted-foreground">Show upcoming events on your dashboard.</p>
            {calendarConnected ? (
              <Badge variant="success" className="gap-1"><CheckCircle2 className="w-3 h-3"/>Connected</Badge>
            ) : (
              <Badge variant="secondary">Not connected</Badge>
            )}
          </div>

          {!calendarConnected && (
            <>
              <p className="text-xs text-muted-foreground">
                Create a project in <a href="https://console.cloud.google.com/apis/credentials" target="_blank" rel="noopener noreferrer" className="text-primary hover:underline">Google Cloud Console <ExternalLink className="w-2.5 h-2.5 inline"/></a>,
                enable the Calendar API, create OAuth 2.0 credentials (Web application), and set the redirect URI to:
              </p>
              <code className="text-xs bg-muted px-2 py-1 rounded block">http://localhost:8080/api/calendar/callback</code>
              <div>
                <label className="text-xs font-medium text-muted-foreground mb-1 block">Client ID</label>
                <Input value={googleClientId} onChange={(e) => setGoogleClientId(e.target.value)} placeholder="xxx.apps.googleusercontent.com" />
              </div>
              <div>
                <label className="text-xs font-medium text-muted-foreground mb-1 block">Client Secret</label>
                <Input type="password" value={googleClientSecret} onChange={(e) => setGoogleClientSecret(e.target.value)} placeholder="GOCSPX-..." />
              </div>
              <div className="flex gap-2">
                <Button variant="outline" size="sm" onClick={async () => {
                  if (!googleClientId || !googleClientSecret) { onToast('Enter Client ID and Secret', 'error'); return; }
                  // Save credentials first
                  await fetch('/api/pulse/settings/app', { method: 'PUT', headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ google_client_id: googleClientId, google_client_secret: googleClientSecret }) });
                  onToast('Credentials saved', 'success');
                }}>Save Credentials</Button>
                <Button size="sm" disabled={!googleClientId || !googleClientSecret || calendarConnecting} onClick={async () => {
                  setCalendarConnecting(true);
                  try {
                    // Save first
                    await fetch('/api/pulse/settings/app', { method: 'PUT', headers: { 'Content-Type': 'application/json' },
                      body: JSON.stringify({ google_client_id: googleClientId, google_client_secret: googleClientSecret }) });
                    const res = await fetch('/api/pulse/calendar/auth-url');
                    if (!res.ok) { onToast('Failed to get auth URL. Check credentials.', 'error'); setCalendarConnecting(false); return; }
                    const { url } = await res.json();
                    const popup = window.open(url, 'google-auth', 'width=500,height=600');
                    const poll = setInterval(async () => {
                      const s = await fetch('/api/pulse/calendar/status').then(r => r.json()).catch(() => ({ connected: false }));
                      if (s.connected) { clearInterval(poll); setCalendarConnected(true); setCalendarConnecting(false); onToast('Google Calendar connected!', 'success'); if (popup) popup.close(); }
                    }, 2000);
                    setTimeout(() => { clearInterval(poll); setCalendarConnecting(false); }, 120000);
                  } catch { setCalendarConnecting(false); onToast('Failed', 'error'); }
                }}>
                  {calendarConnecting ? <Loader2 className="w-3.5 h-3.5 animate-spin mr-1"/> : null}
                  {calendarConnecting ? 'Connecting...' : 'Connect Google Calendar'}
                </Button>
              </div>
            </>
          )}

          {calendarConnected && (
            <Button variant="outline" size="sm" className="text-destructive border-destructive/30 hover:bg-destructive/10" onClick={async () => {
              await fetch('/api/pulse/calendar/disconnect', { method: 'POST' });
              setCalendarConnected(false);
              onToast('Disconnected', 'success');
            }}>Disconnect</Button>
          )}
        </CardContent>
      </Card>

      {/* ZeroClaw Agent */}
      <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-3 flex items-center gap-1.5"><Bot className="w-3.5 h-3.5" />ZeroClaw Agent</h3>
      <Card className="mb-6">
        <CardContent className="p-4 space-y-3">
          <div className="flex items-center justify-between mb-1">
            <p className="text-xs text-muted-foreground">Connect to a running ZeroClaw agent for AI chat.</p>
            {zeroclawStatus?.reachable ? (
              <Badge variant="success" className="gap-1"><CheckCircle2 className="w-3 h-3"/>Reachable</Badge>
            ) : zeroclawStatus?.configured ? (
              <Badge variant="destructive" className="gap-1"><XCircle className="w-3 h-3"/>Unreachable</Badge>
            ) : (
              <Badge variant="secondary">Not configured</Badge>
            )}
          </div>
          <div>
            <label className="text-xs font-medium text-muted-foreground mb-1 block">Gateway URL</label>
            <Input value={zeroclawUrl} onChange={(e) => setZeroclawUrl(e.target.value)} placeholder="http://localhost:42617" />
          </div>
          <div>
            <label className="text-xs font-medium text-muted-foreground mb-1 block">Auth Token (optional)</label>
            <Input type="password" value={zeroclawToken} onChange={(e) => setZeroclawToken(e.target.value)} placeholder="Pairing token or API key" />
          </div>
          <div className="flex gap-2">
            <Button size="sm" onClick={async () => {
              try {
                const res = await fetch('/api/pulse/zeroclaw/config', { method: 'PUT', headers: { 'Content-Type': 'application/json' },
                  body: JSON.stringify({ url: zeroclawUrl, token: zeroclawToken }) });
                if (res.ok) {
                  onToast('ZeroClaw config saved', 'success');
                  const s = await fetch('/api/pulse/zeroclaw/status').then(r => r.json()).catch(() => null);
                  setZeroclawStatus(s);
                  if (s?.reachable) onToast('ZeroClaw is reachable!', 'success');
                  else if (s?.configured) onToast('Saved but ZeroClaw is not reachable at that URL', 'error');
                }
              } catch { onToast('Failed', 'error'); }
            }} disabled={!zeroclawUrl.trim()}>Save & Test</Button>
            {zeroclawStatus?.configured && (
              <Button variant="outline" size="sm" className="text-destructive border-destructive/30 hover:bg-destructive/10" onClick={async () => {
                await fetch('/api/pulse/zeroclaw/config', { method: 'PUT', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ url: '', token: '' }) });
                setZeroclawUrl(''); setZeroclawToken(''); setZeroclawStatus(null);
                onToast('ZeroClaw disconnected', 'success');
              }}>Disconnect</Button>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Collectors */}
      <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-3">Collectors</h3>
      {loading ? (
        <div className="space-y-3 mb-6">{[1, 2].map((i) => <div key={i} className="h-16 rounded-lg bg-card border border-border animate-pulse" />)}</div>
      ) : (
        <div className="space-y-2 mb-6">
          {data?.collectors.map((collector) => {
            const lastRun = data.recent_runs.find((r) => r.collector_id === collector.id);
            return <CollectorRow key={collector.id} collector={collector} lastRun={lastRun}
              onTrigger={() => triggerCollector(collector.id, collector.name)}
              onSaveInterval={(secs) => saveInterval(collector.id, collector.name, secs)} />;
          })}
        </div>
      )}

      {/* Custom RSS feeds */}
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider flex items-center gap-1.5"><Rss className="w-3.5 h-3.5" />Custom RSS Feeds</h3>
        {!showAddFeed && <Button variant="outline" size="sm" onClick={() => setShowAddFeed(true)}><Plus className="w-3.5 h-3.5 mr-1" />Add Feed</Button>}
      </div>
      {showAddFeed && <AddFeedForm onAdd={addFeed} onCancel={() => setShowAddFeed(false)} />}
      {feeds.length > 0 ? (
        <div className="space-y-2">
          {feeds.map((feed) => (
            <Card key={feed.url}><CardContent className="p-3 flex items-center gap-3">
              <Rss className="w-4 h-4 text-primary shrink-0" />
              <div className="flex-1 min-w-0"><div className="text-sm font-medium truncate">{feed.name}</div><div className="text-xs text-muted-foreground truncate">{feed.url}</div></div>
              <Button variant="ghost" size="sm" className="text-destructive hover:text-destructive hover:bg-destructive/10 shrink-0" onClick={() => removeFeed(feed.url, feed.name)}><Trash2 className="w-3.5 h-3.5" /></Button>
            </CardContent></Card>
          ))}
        </div>
      ) : !showAddFeed && (
        <Card><CardContent className="p-4 text-center text-muted-foreground text-sm">No custom feeds. Click "Add Feed" to subscribe.</CardContent></Card>
      )}
    </div>
  );
}

function AddFeedForm({ onAdd, onCancel }: { onAdd: (name: string, url: string) => void; onCancel: () => void }) {
  const [name, setName] = useState(''); const [url, setUrl] = useState(''); const [saving, setSaving] = useState(false);
  const handleSubmit = async () => { if (!name.trim() || !url.trim()) return; setSaving(true); await onAdd(name.trim(), url.trim()); setSaving(false); };
  return (
    <Card className="mb-4"><CardContent className="p-4 space-y-3">
      <div><label className="text-xs font-medium text-muted-foreground mb-1 block">Feed Name</label><Input placeholder="e.g. My Blog" value={name} onChange={(e) => setName(e.target.value)} autoFocus /></div>
      <div><label className="text-xs font-medium text-muted-foreground mb-1 block">Feed URL</label><Input placeholder="https://example.com/feed.xml" value={url} onChange={(e) => setUrl(e.target.value)} onKeyDown={(e) => { if (e.key === 'Enter') handleSubmit(); }} /></div>
      <div className="flex gap-2">
        <Button size="sm" onClick={handleSubmit} disabled={saving || !name.trim() || !url.trim()}>{saving ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : 'Add Feed'}</Button>
        <Button variant="ghost" size="sm" onClick={onCancel}>Cancel</Button>
      </div>
    </CardContent></Card>
  );
}

function CollectorRow({ collector, lastRun, onTrigger, onSaveInterval }: {
  collector: { id: string; name: string; enabled: boolean; interval_secs: number };
  lastRun?: { status: string; items_count: number } | null;
  onTrigger: () => void; onSaveInterval: (secs: number) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [val, setVal] = useState(String(collector.interval_secs));
  const [saving, setSaving] = useState(false);
  const save = async () => { const s = parseInt(val); if (isNaN(s) || s < 10) return; setSaving(true); await onSaveInterval(s); setSaving(false); setEditing(false); };
  return (
    <Card><CardContent className="p-3 flex items-center justify-between gap-3">
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-0.5">
          <span className="text-sm font-medium">{collector.name}</span>
          <Badge variant={collector.enabled ? 'success' : 'secondary'} className="text-[0.6rem]">{collector.enabled ? 'ON' : 'OFF'}</Badge>
        </div>
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <span>Refresh:</span>
          {editing ? (
            <div className="flex items-center gap-1">
              <Input type="number" min="10" value={val} onChange={(e) => setVal(e.target.value)} className="h-5 w-16 text-xs px-1.5" autoFocus
                onKeyDown={(e) => { if (e.key === 'Enter') save(); if (e.key === 'Escape') setEditing(false); }} />
              <span className="text-[0.6rem]">sec</span>
              <button onClick={save} disabled={saving} className="text-success cursor-pointer">{saving ? <Loader2 className="w-3 h-3 animate-spin" /> : <Check className="w-3 h-3" />}</button>
              <button onClick={() => setEditing(false)} className="text-muted-foreground cursor-pointer"><X className="w-3 h-3" /></button>
            </div>
          ) : (
            <button onClick={() => setEditing(true)} className="flex items-center gap-0.5 hover:text-primary cursor-pointer group">
              <span className="font-medium">{formatInterval(collector.interval_secs)}</span>
              <Pencil className="w-2.5 h-2.5 opacity-0 group-hover:opacity-100" />
            </button>
          )}
          {lastRun && <><Badge variant={lastRun.status === 'success' ? 'success' : 'destructive'} className="text-[0.55rem] px-1 py-0">{lastRun.status}</Badge><span>{lastRun.items_count} items</span></>}
        </div>
      </div>
      {collector.enabled && <Button variant="outline" size="sm" onClick={onTrigger}>Run Now</Button>}
    </CardContent></Card>
  );
}

function AddVideoForm({ onAdd, onCancel }: { onAdd: (platform: string, channelId: string, name: string) => void; onCancel: () => void }) {
  const [platform, setPlatform] = useState('youtube');
  const [channelId, setChannelId] = useState('');
  const [name, setName] = useState('');
  const [saving, setSaving] = useState(false);
  const handleSubmit = async () => { if (!channelId.trim() || !name.trim()) return; setSaving(true); await onAdd(platform, channelId.trim(), name.trim()); setSaving(false); };
  return (
    <div className="space-y-3 p-3 border border-border rounded-lg mb-3">
      <div>
        <label className="text-xs font-medium text-muted-foreground mb-1 block">Platform</label>
        <select value={platform} onChange={(e) => setPlatform(e.target.value)}
          className="h-8 w-full rounded-md border border-input bg-transparent px-3 text-sm text-foreground">
          <option value="youtube">YouTube</option>
          <option value="bilibili">Bilibili</option>
        </select>
      </div>
      <div>
        <label className="text-xs font-medium text-muted-foreground mb-1 block">
          {platform === 'youtube' ? 'Channel ID' : 'UP主 UID'}
        </label>
        <Input value={channelId} onChange={(e) => setChannelId(e.target.value)}
          placeholder={platform === 'youtube' ? 'UCxxxxxx (from channel URL)' : 'e.g. 12345678'} />
        <p className="text-xs text-muted-foreground mt-1">
          {platform === 'youtube'
            ? 'Find it in the channel URL: youtube.com/channel/UCxxxxxx'
            : 'Find it in the UP主 space URL: space.bilibili.com/12345678'}
        </p>
      </div>
      <div>
        <label className="text-xs font-medium text-muted-foreground mb-1 block">Display Name</label>
        <Input value={name} onChange={(e) => setName(e.target.value)} placeholder="e.g. Linus Tech Tips"
          onKeyDown={(e) => { if (e.key === 'Enter') handleSubmit(); }} />
      </div>
      <div className="flex gap-2">
        <Button size="sm" onClick={handleSubmit} disabled={saving || !channelId.trim() || !name.trim()}>
          {saving ? <Loader2 className="w-3.5 h-3.5 animate-spin"/> : 'Add Channel'}
        </Button>
        <Button variant="ghost" size="sm" onClick={onCancel}>Cancel</Button>
      </div>
    </div>
  );
}

function formatInterval(s: number) { return s < 60 ? `${s}s` : s < 3600 ? `${Math.round(s/60)}m` : `${(s/3600).toFixed(1).replace(/\.0$/,'')}h`; }
