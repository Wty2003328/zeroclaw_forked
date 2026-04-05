import { useState, useEffect, useCallback } from 'react';

export function useWidgetData<T>(url: string, refreshInterval: number = 60000) {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    try {
      const response = await fetch(url);
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }
      const json = await response.json();
      setData(json);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setLoading(false);
    }
  }, [url]);

  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, refreshInterval);

    // Listen for manual refresh events (e.g. after "Run Now" in collectors)
    const handleRefresh = () => fetchData();
    window.addEventListener('pulse-data-refresh', handleRefresh);

    return () => {
      clearInterval(interval);
      window.removeEventListener('pulse-data-refresh', handleRefresh);
    };
  }, [fetchData, refreshInterval]);

  return { data, loading, error, refetch: fetchData };
}
