import { useWidgetData } from '../../../hooks/useWidgetData';
import { Droplets, Wind, Sun, Thermometer, CloudRain, Cloud, Gauge, Sunrise, Sunset, Moon, Eye } from 'lucide-react';
import type { FeedResponse } from '../../../types/pulse';

interface Hourly { time: string; temp_f?: string; rain_chance: string }
interface Forecast { date: string; high_f: string; low_f: string; description?: string; rain_chance?: string; sunrise?: string; sunset?: string; moon_phase?: string; hourly?: Hourly[] }
interface WMeta {
  location?: string; temp_f: number; temp_c: number; description: string;
  feels_like_f?: number; humidity?: number; wind_speed_mph?: number; wind_speed_kmph?: number;
  wind_direction?: string; visibility_km?: string; uv_index?: number;
  pressure_mb?: string; cloud_cover?: string; forecast?: Forecast[];
}

function fmtDay(d: string) { try{const dt=new Date(d+'T00:00:00'),t=new Date();t.setHours(0,0,0,0);const diff=(dt.getTime()-t.getTime())/864e5;if(diff<1)return'Today';if(diff<2)return'Tmrw';return dt.toLocaleDateString('en-US',{weekday:'short'});}catch{return d;} }

interface Props { dims?: any }

export default function Weather({ }: Props) {
  const { data, loading, error } = useWidgetData<FeedResponse>('/api/pulse/feed?source=weather&limit=1', 300000);

  if (loading) return <div className="flex-1 flex items-center justify-center text-muted-foreground cq-text-sm">Loading...</div>;
  if (error) return <div className="flex-1 flex items-center justify-center text-destructive cq-text-sm">Error</div>;
  if (!data || data.items.length === 0) return <div className="flex-1 flex items-center justify-center text-muted-foreground cq-text-sm">No data</div>;

  const m = data.items[0]?.metadata as unknown as WMeta;
  const mph = m.wind_speed_mph ?? (m.wind_speed_kmph != null ? Math.round(m.wind_speed_kmph * 0.621) : null);
  const fc = m.forecast || [];
  const todayHourly = fc[0]?.hourly || [];

  return (
    <div className="flex flex-col h-full overflow-hidden gap-1">
      {/* Temp + description — always */}
      <div className="flex items-center justify-between shrink-0">
        <span className="cq-text-4xl font-bold leading-none">{Math.round(m.temp_f)}°</span>
        <div className="text-right min-w-0 ml-1">
          <div className="cq-text-sm font-medium truncate">{m.description}</div>
          <div className="hidden @[160px]:block cq-text-xs text-muted-foreground truncate">{m.location}</div>
        </div>
      </div>

      {/* Stats — simple readable text, wraps naturally */}
      <div className="flex flex-wrap gap-x-3 gap-y-0.5 cq-text-xs text-muted-foreground shrink-0">
        {m.humidity != null && <span><Droplets className="w-3 h-3 text-blue-400 inline mr-0.5"/>{m.humidity}%</span>}
        {mph != null && <span><Wind className="w-3 h-3 text-cyan-400 inline mr-0.5"/>{mph}mph</span>}
        {m.feels_like_f != null && <span><Thermometer className="w-3 h-3 text-orange-400 inline mr-0.5"/>{Math.round(m.feels_like_f)}°</span>}
        {m.uv_index != null && <span className="hidden cqh-140"><Sun className="w-3 h-3 text-yellow-400 inline mr-0.5"/>UV{m.uv_index}</span>}
        {m.pressure_mb && <span className="hidden @[220px]:inline"><Gauge className="w-3 h-3 text-purple-400 inline mr-0.5"/>{m.pressure_mb}mb</span>}
        {m.cloud_cover && <span className="hidden @[220px]:inline"><Cloud className="w-3 h-3 text-gray-400 inline mr-0.5"/>{m.cloud_cover}%</span>}
        {m.visibility_km && <span className="hidden @[300px]:inline"><Eye className="w-3 h-3 text-emerald-400 inline mr-0.5"/>{m.visibility_km}km</span>}
      </div>

      {/* Sunrise/sunset */}
      {fc[0]?.sunrise && (
        <div className="hidden cqh-180 items-center gap-2 cq-text-xs text-muted-foreground shrink-0">
          <Sunrise className="w-3 h-3 text-orange-300 shrink-0"/><span>{fc[0].sunrise}</span>
          <Sunset className="w-3 h-3 text-red-300 shrink-0"/><span>{fc[0].sunset}</span>
          {fc[0].moon_phase && <span className="hidden @[280px]:inline ml-auto"><Moon className="w-3 h-3 text-blue-200 inline mr-0.5"/>{fc[0].moon_phase}</span>}
        </div>
      )}

      {/* Hourly */}
      {todayHourly.length > 0 && (
        <div className="hidden cqwh-300-200-block shrink-0">
          <div className="flex gap-0.5 overflow-x-auto pb-0.5">
            {todayHourly.map((h, i) => (
              <div key={i} className="flex flex-col items-center shrink-0 px-1 py-0.5 bg-muted rounded min-w-[2rem]">
                <span className="text-muted-foreground" style={{fontSize:'0.5rem'}}>{h.time}</span>
                <span className="cq-text-xs font-bold">{h.temp_f}°</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Forecast — always visible, fills remaining space */}
      {fc.length > 0 && (
        <div className="flex flex-col flex-1 overflow-y-auto min-h-0">
          {/* Compact forecast at tiny sizes */}
          <div className="@[200px]:hidden flex flex-col flex-1 justify-evenly">
            {fc.map((day, i) => (
              <div key={i} className="flex items-center justify-between cq-text-xs">
                <span className="font-semibold text-muted-foreground">{fmtDay(day.date)}</span>
                <span><span className="text-destructive/80 font-bold">{day.high_f}°</span> <span className="text-muted-foreground">{day.low_f}°</span></span>
              </div>
            ))}
          </div>
          {/* Full forecast at wider */}
          <div className="hidden @[200px]:block flex-1 overflow-y-auto">
            <div className="cq-text-xs font-semibold text-muted-foreground mb-1">Forecast</div>
            <div className="flex flex-col gap-1">
              {fc.map((day, i) => {
                const rain = day.rain_chance ? parseInt(day.rain_chance) : 0;
                return (
                  <div key={i} className="p-2 bg-muted rounded-lg">
                    <div className="flex items-center justify-between cq-text-sm">
                      <span className="font-semibold">{fmtDay(day.date)}</span>
                      <div className="flex items-center gap-2">
                        {rain > 0 && <span className="text-blue-400"><CloudRain className="w-3 h-3 inline mr-0.5"/>{rain}%</span>}
                        <span className="text-destructive/80 font-bold">{day.high_f}°</span>
                        <span className="text-muted-foreground">{day.low_f}°</span>
                      </div>
                    </div>
                    {day.description && <div className="cq-text-xs text-muted-foreground mt-0.5">{day.description}</div>}
                    {day.sunrise && <div className="hidden cqh-200 items-center gap-2 mt-1 cq-text-xs text-muted-foreground">
                      <Sunrise className="w-3 h-3 text-orange-300 inline"/>{day.sunrise}
                      <Sunset className="w-3 h-3 text-red-300 inline ml-1"/>{day.sunset}
                    </div>}
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
