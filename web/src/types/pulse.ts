export interface FeedItem {
  id: string;
  source: string;
  collector_id: string;
  title: string;
  url: string | null;
  content: string | null;
  summary: string | null;
  metadata: Record<string, unknown>;
  tags: string[];
  score: number | null;
  published_at: string | null;
  collected_at: string;
}

export interface FeedResponse {
  items: FeedItem[];
  count: number;
  limit: number;
  offset: number;
}

export interface CollectorInfo {
  id: string;
  name: string;
  enabled: boolean;
  interval_secs: number;
}

export interface CollectorRun {
  id: string;
  collector_id: string;
  started_at: string;
  finished_at: string | null;
  items_count: number;
  status: string;
  error: string | null;
}

export interface CollectorsResponse {
  collectors: CollectorInfo[];
  recent_runs: CollectorRun[];
}

export interface ProviderSetting {
  id: string;
  display_name: string;
  api_key_set: boolean;
  api_key_preview: string | null;
  model: string | null;
  endpoint: string | null;
  enabled: boolean;
  is_active: boolean;
  extra_config: Record<string, string>;
}

export interface ProviderTestResult {
  success: boolean;
  message: string;
  model_used?: string;
}

export interface ProvidersResponse {
  providers: ProviderSetting[];
}
