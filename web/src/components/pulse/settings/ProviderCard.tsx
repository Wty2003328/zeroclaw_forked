import { useState } from 'react';
import { Eye, EyeOff, Loader2, CheckCircle2, XCircle, Zap, ChevronDown, ChevronUp } from 'lucide-react';
import { Card, CardContent, CardHeader } from '../ui/card';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Badge } from '../ui/badge';
import type { ProviderMeta } from '../../../lib/providers';
import type { ProviderSetting, ProviderTestResult } from '../../../types/pulse';

interface ProviderCardProps {
  meta: ProviderMeta;
  setting?: ProviderSetting;
  onSaved: () => void;
  onToast: (message: string, type: 'success' | 'error') => void;
}

export function ProviderCard({ meta, setting, onSaved, onToast }: ProviderCardProps) {
  const [apiKey, setApiKey] = useState('');
  const [model, setModel] = useState(setting?.model || meta.defaultModels[0] || '');
  const [endpoint, setEndpoint] = useState(setting?.endpoint || '');
  const [showKey, setShowKey] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<ProviderTestResult | null>(null);
  const [saving, setSaving] = useState(false);

  const isConfigured = setting?.api_key_set ?? false;
  const isActive = setting?.is_active ?? false;

  const handleSave = async () => {
    if (!apiKey && !isConfigured) {
      onToast('Please enter an API key', 'error');
      return;
    }

    setSaving(true);
    try {
      const body: Record<string, string> = { model };
      if (apiKey) body.api_key = apiKey;
      if (endpoint) body.endpoint = endpoint;

      const res = await fetch(`/api/settings/providers/${meta.id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });

      if (res.ok) {
        setApiKey('');
        setTestResult(null);
        onSaved();
        onToast(`${meta.name} configuration saved`, 'success');
      } else {
        const err = await res.text();
        onToast(`Failed to save: ${err}`, 'error');
      }
    } catch {
      onToast('Failed to save configuration', 'error');
    } finally {
      setSaving(false);
    }
  };

  const handleTest = async () => {
    setTesting(true);
    setTestResult(null);
    try {
      const body: Record<string, string> = { model };
      if (apiKey) body.api_key = apiKey;

      const res = await fetch(`/api/settings/providers/${meta.id}/test`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });

      const result: ProviderTestResult = await res.json();
      setTestResult(result);

      if (result.success) {
        onToast(`${meta.name} connection successful`, 'success');
      } else {
        onToast(`${meta.name} test failed: ${result.message}`, 'error');
      }
    } catch {
      setTestResult({ success: false, message: 'Network error' });
      onToast('Connection test failed', 'error');
    } finally {
      setTesting(false);
    }
  };

  const handleActivate = async () => {
    try {
      const res = await fetch(`/api/settings/providers/${meta.id}/activate`, {
        method: 'POST',
      });

      if (res.ok) {
        onSaved();
        onToast(`${meta.name} is now the active provider`, 'success');
      } else {
        const err = await res.text();
        onToast(`Failed to activate: ${err}`, 'error');
      }
    } catch {
      onToast('Failed to activate provider', 'error');
    }
  };

  const handleDelete = async () => {
    try {
      const res = await fetch(`/api/settings/providers/${meta.id}`, {
        method: 'DELETE',
      });

      if (res.ok) {
        setApiKey('');
        setTestResult(null);
        onSaved();
        onToast(`${meta.name} credentials removed`, 'success');
      }
    } catch {
      onToast('Failed to remove credentials', 'error');
    }
  };

  return (
    <Card className="flex flex-col transition-shadow hover:shadow-md">
      <CardHeader className="pb-3">
        <div className="flex items-start justify-between">
          <div className="flex items-center gap-3">
            <div
              className="w-9 h-9 rounded-lg flex items-center justify-center text-sm font-bold text-white"
              style={{ backgroundColor: meta.color }}
            >
              {meta.name[0]}
            </div>
            <div>
              <h3 className="font-semibold text-foreground text-sm">{meta.name}</h3>
              <p className="text-xs text-muted-foreground">{meta.company}</p>
            </div>
          </div>
          {isActive ? (
            <Badge variant="success" className="gap-1">
              <Zap className="w-3 h-3" /> Active
            </Badge>
          ) : isConfigured ? (
            <Badge variant="secondary">Configured</Badge>
          ) : (
            <Badge variant="outline" className="text-muted-foreground">Not Set</Badge>
          )}
        </div>
        <p className="text-xs text-muted-foreground mt-2">{meta.description}</p>
      </CardHeader>

      <CardContent className="flex flex-col gap-3 flex-1">
        {/* API Key */}
        <div>
          <label className="text-xs font-medium text-muted-foreground mb-1 block">API Key</label>
          <div className="relative">
            <Input
              type={showKey ? 'text' : 'password'}
              placeholder={isConfigured ? setting?.api_key_preview || '••••••••' : meta.authHint}
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              className="pr-9"
            />
            <button
              type="button"
              onClick={() => setShowKey(!showKey)}
              className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground transition-colors"
            >
              {showKey ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
            </button>
          </div>
        </div>

        {/* Model */}
        <div>
          <label className="text-xs font-medium text-muted-foreground mb-1 block">Model</label>
          <select
            value={model}
            onChange={(e) => setModel(e.target.value)}
            className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm text-foreground shadow-sm transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
          >
            {meta.defaultModels.map((m) => (
              <option key={m} value={m}>{m}</option>
            ))}
          </select>
        </div>

        {/* Advanced (endpoint override) */}
        <button
          type="button"
          onClick={() => setShowAdvanced(!showAdvanced)}
          className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
        >
          {showAdvanced ? <ChevronUp className="w-3 h-3" /> : <ChevronDown className="w-3 h-3" />}
          Advanced
        </button>
        {showAdvanced && (
          <div>
            <label className="text-xs font-medium text-muted-foreground mb-1 block">Custom Endpoint</label>
            <Input
              type="text"
              placeholder={meta.defaultEndpoint}
              value={endpoint}
              onChange={(e) => setEndpoint(e.target.value)}
            />
          </div>
        )}

        {/* Test result */}
        {testResult && (
          <div className={`flex items-center gap-2 text-xs px-3 py-2 rounded-md ${
            testResult.success
              ? 'bg-success/10 text-success'
              : 'bg-destructive/10 text-destructive'
          }`}>
            {testResult.success ? <CheckCircle2 className="w-3.5 h-3.5" /> : <XCircle className="w-3.5 h-3.5" />}
            {testResult.message}
          </div>
        )}

        {/* Actions */}
        <div className="flex gap-2 mt-auto pt-2">
          <Button
            variant="outline"
            size="sm"
            className="flex-1"
            onClick={handleTest}
            disabled={testing || (!apiKey && !isConfigured)}
          >
            {testing ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : 'Test'}
          </Button>
          <Button
            size="sm"
            className="flex-1"
            onClick={handleSave}
            disabled={saving || (!apiKey && !isConfigured)}
          >
            {saving ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : 'Save'}
          </Button>
        </div>

        {isConfigured && (
          <div className="flex gap-2">
            {!isActive && (
              <Button variant="outline" size="sm" className="flex-1 text-primary border-primary/30 hover:bg-primary/10" onClick={handleActivate}>
                Set as Active
              </Button>
            )}
            <Button variant="ghost" size="sm" className="text-destructive hover:text-destructive hover:bg-destructive/10" onClick={handleDelete}>
              Remove
            </Button>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
