import { useState, useEffect } from 'react';
import { Plus, Zap, Trash2, CheckCircle2, XCircle, Loader2, Eye, EyeOff, ChevronDown, ChevronUp, Pencil } from 'lucide-react';
import { cn } from '../../../lib/pulseUtils';
import { Card, CardContent } from '../ui/card';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Badge } from '../ui/badge';
import { PROVIDERS, type ProviderMeta } from '../../../lib/providers';
import type { ProviderSetting, ProviderTestResult } from '../../../types/pulse';

interface Props {
  onToast: (message: string, type: 'success' | 'error') => void;
}

export function ModelsPanel({ onToast }: Props) {
  const [providers, setProviders] = useState<ProviderSetting[]>([]);
  const [loading, setLoading] = useState(true);
  const [addingProvider, setAddingProvider] = useState<string | null>(null);

  const fetchProviders = async () => {
    try {
      const res = await fetch('/api/pulse/settings/providers');
      if (res.ok) {
        const data = await res.json();
        setProviders(data.providers);
      }
    } catch {
      setProviders([]);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { fetchProviders(); }, []);

  const configured = providers.filter((p) => p.api_key_set);
  const available = PROVIDERS.filter((meta) => !configured.some((p) => p.id === meta.id));

  const getMeta = (id: string) => PROVIDERS.find((p) => p.id === id);

  if (loading) {
    return (
      <div>
        <h2 className="text-lg font-semibold mb-1">AI Models</h2>
        <p className="text-sm text-muted-foreground mb-6">Manage AI providers for the intelligence pipeline.</p>
        <div className="space-y-3">
          {[1, 2, 3].map((i) => <div key={i} className="h-20 rounded-lg bg-card border border-border animate-pulse" />)}
        </div>
      </div>
    );
  }

  return (
    <div>
      <h2 className="text-lg font-semibold mb-1">AI Models</h2>
      <p className="text-sm text-muted-foreground mb-6">
        Configure API keys for AI providers. Set one as active to power scoring, summarization, and tagging.
      </p>

      {/* Configured models */}
      {configured.length > 0 && (
        <div className="mb-8">
          <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-3">Configured</h3>
          <div className="space-y-2">
            {configured.map((setting) => {
              const meta = getMeta(setting.id);
              if (!meta) return null;
              return (
                <ConfiguredModelRow
                  key={setting.id}
                  meta={meta}
                  setting={setting}
                  onSaved={fetchProviders}
                  onToast={onToast}
                />
              );
            })}
          </div>
        </div>
      )}

      {/* Add new model form */}
      {addingProvider && (
        <div className="mb-8">
          <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-3">Add Provider</h3>
          <ProviderForm
            meta={PROVIDERS.find((p) => p.id === addingProvider)!}
            onSaved={() => { setAddingProvider(null); fetchProviders(); }}
            onCancel={() => setAddingProvider(null)}
            onToast={onToast}
          />
        </div>
      )}

      {/* Available providers to add */}
      {available.length > 0 && !addingProvider && (
        <div>
          <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-3">Available Providers</h3>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
            {available.map((meta) => (
              <button
                key={meta.id}
                onClick={() => setAddingProvider(meta.id)}
                className="flex items-center gap-3 p-3 rounded-lg border border-border bg-card hover:border-primary/40 hover:bg-primary/5 transition-colors text-left cursor-pointer group"
              >
                <div
                  className="w-8 h-8 rounded-md flex items-center justify-center text-xs font-bold text-white shrink-0"
                  style={{ backgroundColor: meta.color }}
                >
                  {meta.name[0]}
                </div>
                <div className="flex-1 min-w-0">
                  <div className="text-sm font-medium text-foreground">{meta.name}</div>
                  <div className="text-xs text-muted-foreground truncate">{meta.company}</div>
                </div>
                <Plus className="w-4 h-4 text-muted-foreground group-hover:text-primary transition-colors shrink-0" />
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

/* ── Configured Model Row ── */

function ConfiguredModelRow({
  meta, setting, onSaved, onToast,
}: {
  meta: ProviderMeta; setting: ProviderSetting; onSaved: () => void;
  onToast: (msg: string, type: 'success' | 'error') => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const [editing, setEditing] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<ProviderTestResult | null>(null);

  const handleTest = async () => {
    setTesting(true); setTestResult(null);
    try {
      const res = await fetch(`/api/settings/providers/${meta.id}/test`, {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ model: setting.model }),
      });
      const result: ProviderTestResult = await res.json();
      setTestResult(result);
      onToast(result.success ? `${meta.name} connected` : `${meta.name}: ${result.message}`, result.success ? 'success' : 'error');
    } catch { setTestResult({ success: false, message: 'Network error' }); }
    finally { setTesting(false); }
  };

  const handleActivate = async () => {
    try {
      const res = await fetch(`/api/settings/providers/${meta.id}/activate`, { method: 'POST' });
      if (res.ok) { onSaved(); onToast(`${meta.name} is now active`, 'success'); }
    } catch { onToast('Failed to activate', 'error'); }
  };

  const handleDelete = async () => {
    try {
      const res = await fetch(`/api/settings/providers/${meta.id}`, { method: 'DELETE' });
      if (res.ok) { onSaved(); onToast(`${meta.name} removed`, 'success'); }
    } catch { onToast('Failed to remove', 'error'); }
  };

  if (editing) {
    return (
      <ProviderForm
        meta={meta}
        existing={setting}
        onSaved={() => { setEditing(false); onSaved(); }}
        onCancel={() => setEditing(false)}
        onToast={onToast}
      />
    );
  }

  return (
    <Card>
      <div
        className="flex items-center gap-3 p-3 cursor-pointer hover:bg-accent/30 transition-colors rounded-lg"
        onClick={() => setExpanded(!expanded)}
      >
        <div className="w-8 h-8 rounded-md flex items-center justify-center text-xs font-bold text-white shrink-0" style={{ backgroundColor: meta.color }}>
          {meta.name[0]}
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium">{meta.name}</span>
            <span className="text-xs text-muted-foreground">{meta.company}</span>
          </div>
          <div className="text-xs text-muted-foreground">
            {setting.model || meta.defaultModels[0] || ''}
            {setting.api_key_preview && <span className="ml-2 opacity-60">{setting.api_key_preview}</span>}
          </div>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          {testResult && (testResult.success ? <CheckCircle2 className="w-4 h-4 text-success" /> : <XCircle className="w-4 h-4 text-destructive" />)}
          {setting.is_active ? (
            <Badge variant="success" className="gap-1 text-[0.6rem]"><Zap className="w-3 h-3" />Active</Badge>
          ) : (
            <Badge variant="secondary" className="text-[0.6rem]">Ready</Badge>
          )}
          {expanded ? <ChevronUp className="w-4 h-4 text-muted-foreground" /> : <ChevronDown className="w-4 h-4 text-muted-foreground" />}
        </div>
      </div>

      {expanded && (
        <CardContent className="pt-0 pb-3 px-3 border-t border-border">
          <div className="flex flex-wrap gap-2 pt-3">
            <Button variant="outline" size="sm" onClick={(e) => { e.stopPropagation(); setEditing(true); }}>
              <Pencil className="w-3.5 h-3.5 mr-1" /> Edit
            </Button>
            <Button variant="outline" size="sm" onClick={handleTest} disabled={testing}>
              {testing ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : 'Test'}
            </Button>
            {!setting.is_active && (
              <Button variant="outline" size="sm" className="text-primary border-primary/30 hover:bg-primary/10" onClick={handleActivate}>
                Set as Active
              </Button>
            )}
            <Button variant="ghost" size="sm" className="text-destructive hover:text-destructive hover:bg-destructive/10 ml-auto" onClick={handleDelete}>
              <Trash2 className="w-3.5 h-3.5 mr-1" /> Remove
            </Button>
          </div>
          {testResult && !testResult.success && (
            <div className="mt-2 text-xs text-destructive bg-destructive/10 px-3 py-2 rounded-md">{testResult.message}</div>
          )}
        </CardContent>
      )}
    </Card>
  );
}

/* ── Provider Form (add or edit) ── */

function ProviderForm({
  meta, existing, onSaved, onCancel, onToast,
}: {
  meta: ProviderMeta; existing?: ProviderSetting; onSaved: () => void; onCancel: () => void;
  onToast: (msg: string, type: 'success' | 'error') => void;
}) {
  const [apiKey, setApiKey] = useState('');
  const [model, setModel] = useState(existing?.model || meta.defaultModels[0] || '');
  const [endpoint, setEndpoint] = useState(existing?.endpoint || '');
  const [showKey, setShowKey] = useState(false);
  const [showEndpoint, setShowEndpoint] = useState(!!existing?.endpoint);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<ProviderTestResult | null>(null);
  const [saving, setSaving] = useState(false);
  const isEdit = !!existing;

  const handleTest = async () => {
    if (!apiKey && !existing?.api_key_set) { onToast('Enter an API key first', 'error'); return; }
    setTesting(true); setTestResult(null);
    try {
      const body: Record<string, string> = { model };
      if (apiKey) body.api_key = apiKey;
      const res = await fetch(`/api/settings/providers/${meta.id}/test`, {
        method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body),
      });
      const result: ProviderTestResult = await res.json();
      setTestResult(result);
      onToast(result.success ? `${meta.name} connected` : `Test failed: ${result.message}`, result.success ? 'success' : 'error');
    } catch { setTestResult({ success: false, message: 'Network error' }); }
    finally { setTesting(false); }
  };

  const handleSave = async () => {
    if (!apiKey && !existing?.api_key_set) { onToast('Enter an API key', 'error'); return; }
    setSaving(true);
    try {
      const body: Record<string, string> = { model };
      if (apiKey) body.api_key = apiKey;
      if (endpoint) body.endpoint = endpoint;
      const res = await fetch(`/api/settings/providers/${meta.id}`, {
        method: 'PUT', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body),
      });
      if (res.ok) { onToast(`${meta.name} ${isEdit ? 'updated' : 'added'}`, 'success'); onSaved(); }
      else { onToast(`Failed: ${await res.text()}`, 'error'); }
    } catch { onToast('Failed to save', 'error'); }
    finally { setSaving(false); }
  };

  return (
    <Card>
      <CardContent className="p-4 space-y-4">
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-lg flex items-center justify-center text-sm font-bold text-white" style={{ backgroundColor: meta.color }}>
            {meta.name[0]}
          </div>
          <div>
            <h4 className="text-sm font-semibold">{meta.name}</h4>
            <p className="text-xs text-muted-foreground">{meta.description}</p>
          </div>
        </div>

        <div>
          <label className="text-xs font-medium text-muted-foreground mb-1.5 block">
            API Key {isEdit && <span className="text-muted-foreground/60">(leave blank to keep current)</span>}
          </label>
          <div className="relative">
            <Input
              type={showKey ? 'text' : 'password'}
              placeholder={existing?.api_key_preview || meta.authHint}
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              className="pr-9"
              autoFocus={!isEdit}
            />
            <button type="button" onClick={() => setShowKey(!showKey)} className="absolute right-2.5 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground transition-colors">
              {showKey ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
            </button>
          </div>
        </div>

        <div>
          <label className="text-xs font-medium text-muted-foreground mb-1.5 block">Model</label>
          <select
            value={model} onChange={(e) => setModel(e.target.value)}
            className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm text-foreground shadow-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
          >
            {meta.defaultModels.map((m) => <option key={m} value={m}>{m}</option>)}
          </select>
        </div>

        <button type="button" onClick={() => setShowEndpoint(!showEndpoint)} className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors cursor-pointer">
          {showEndpoint ? <ChevronUp className="w-3 h-3" /> : <ChevronDown className="w-3 h-3" />} Custom endpoint
        </button>
        {showEndpoint && (
          <Input type="text" placeholder={meta.defaultEndpoint} value={endpoint} onChange={(e) => setEndpoint(e.target.value)} />
        )}

        {testResult && (
          <div className={cn('flex items-center gap-2 text-xs px-3 py-2 rounded-md', testResult.success ? 'bg-success/10 text-success' : 'bg-destructive/10 text-destructive')}>
            {testResult.success ? <CheckCircle2 className="w-3.5 h-3.5 shrink-0" /> : <XCircle className="w-3.5 h-3.5 shrink-0" />}
            <span className="break-all">{testResult.message}</span>
          </div>
        )}

        <div className="flex gap-2 pt-1">
          <Button variant="outline" size="sm" onClick={handleTest} disabled={testing || (!apiKey && !existing?.api_key_set)}>
            {testing ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : 'Test Connection'}
          </Button>
          <Button size="sm" onClick={handleSave} disabled={saving || (!apiKey && !existing?.api_key_set)}>
            {saving ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : isEdit ? 'Save Changes' : 'Add Provider'}
          </Button>
          <Button variant="ghost" size="sm" onClick={onCancel}>Cancel</Button>
        </div>
      </CardContent>
    </Card>
  );
}
