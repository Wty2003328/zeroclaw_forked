import { useState, useEffect } from 'react';
import { Link } from 'react-router-dom';
import { ArrowLeft, Cpu, LayoutDashboard, Rss, Info } from 'lucide-react';
import { cn } from '../../lib/pulseUtils';
import { ModelsPanel } from './settings/ModelsPanel';
import { GeneralPanel } from './settings/GeneralPanel';
import { DataSourcesPanel } from './settings/DataSourcesPanel';
import { AboutPanel } from './settings/AboutPanel';

type SettingsTab = 'general' | 'models' | 'sources' | 'about';

const TABS: { id: SettingsTab; label: string; icon: React.ElementType }[] = [
  { id: 'general', label: 'General', icon: LayoutDashboard },
  { id: 'models', label: 'AI Models', icon: Cpu },
  { id: 'sources', label: 'Data Sources', icon: Rss },
  { id: 'about', label: 'About', icon: Info },
];

export default function Settings() {
  const [activeTab, setActiveTab] = useState<SettingsTab>('models');
  const [toast, setToast] = useState<{ message: string; type: 'success' | 'error' } | null>(null);

  useEffect(() => {
    if (toast) {
      const timer = setTimeout(() => setToast(null), 3000);
      return () => clearTimeout(timer);
    }
  }, [toast]);

  const showToast = (message: string, type: 'success' | 'error') => {
    setToast({ message, type });
  };

  return (
    <div className="min-h-screen bg-background flex flex-col">
      <header className="flex items-center gap-4 px-6 py-3 bg-card border-b border-border shrink-0">
        <Link
          to="/"
          className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors no-underline"
        >
          <ArrowLeft className="w-4 h-4" />
          <span>Dashboard</span>
        </Link>
        <div className="h-4 w-px bg-border" />
        <h1 className="text-lg font-bold tracking-tight text-foreground">Settings</h1>
      </header>

      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar */}
        <nav className="w-56 shrink-0 border-r border-border bg-card/50 p-3 flex flex-col gap-1">
          {TABS.map((tab) => {
            const Icon = tab.icon;
            return (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={cn(
                  'flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors text-left w-full cursor-pointer',
                  activeTab === tab.id
                    ? 'bg-primary/10 text-primary'
                    : 'text-muted-foreground hover:text-foreground hover:bg-accent/50'
                )}
              >
                <Icon className="w-4 h-4 shrink-0" />
                {tab.label}
              </button>
            );
          })}
        </nav>

        {/* Content */}
        <main className="flex-1 overflow-y-auto p-6">
          <div className="max-w-3xl">
            {activeTab === 'general' && <GeneralPanel onToast={showToast} />}
            {activeTab === 'models' && <ModelsPanel onToast={showToast} />}
            {activeTab === 'sources' && <DataSourcesPanel onToast={showToast} />}
            {activeTab === 'about' && <AboutPanel />}
          </div>
        </main>
      </div>

      {/* Toast */}
      {toast && (
        <div className={cn(
          'fixed bottom-6 right-6 px-4 py-3 rounded-lg shadow-lg text-sm font-medium z-50',
          toast.type === 'success'
            ? 'bg-success/15 text-success border border-success/30'
            : 'bg-destructive/15 text-destructive border border-destructive/30'
        )}>
          {toast.message}
        </div>
      )}
    </div>
  );
}
