use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use std::time::Duration;

use super::{parse_interval, Collector};
use crate::pulse::config::WeatherConfig;
use crate::pulse::models::RawItem;

pub struct WeatherCollector {
    config: WeatherConfig,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct WttrResponse {
    current_condition: Vec<WttrCondition>,
    nearest_area: Option<Vec<WttrArea>>,
    weather: Option<Vec<WttrForecast>>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct WttrCondition {
    temp_C: String,
    temp_F: String,
    #[serde(rename = "FeelsLikeC")]
    feels_like_c: String,
    #[serde(rename = "FeelsLikeF")]
    feels_like_f: String,
    humidity: String,
    #[serde(rename = "weatherDesc")]
    weather_desc: Vec<WttrValue>,
    #[serde(rename = "windspeedKmph")]
    windspeed_kmph: String,
    #[serde(rename = "windspeedMiles")]
    windspeed_miles: Option<String>,
    #[serde(rename = "winddir16Point")]
    wind_dir: String,
    #[serde(rename = "winddirDegree")]
    winddir_degree: Option<String>,
    visibility: String,
    #[serde(rename = "visibilityMiles")]
    visibility_miles: Option<String>,
    #[serde(rename = "uvIndex")]
    uv_index: String,
    pressure: Option<String>,
    #[serde(rename = "pressureInches")]
    pressure_inches: Option<String>,
    cloudcover: Option<String>,
    #[serde(rename = "precipMM")]
    precip_mm: Option<String>,
    #[serde(rename = "precipInches")]
    precip_inches: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WttrValue {
    value: String,
}

#[derive(Debug, Deserialize)]
struct WttrArea {
    #[serde(rename = "areaName")]
    area_name: Vec<WttrValue>,
    region: Vec<WttrValue>,
    country: Vec<WttrValue>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct WttrForecast {
    #[serde(rename = "maxtempC")]
    max_temp_c: String,
    #[serde(rename = "mintempC")]
    min_temp_c: String,
    #[serde(rename = "maxtempF")]
    max_temp_f: String,
    #[serde(rename = "mintempF")]
    min_temp_f: String,
    date: String,
    #[serde(rename = "avgtempF")]
    avg_temp_f: Option<String>,
    #[serde(rename = "sunHour")]
    sun_hour: Option<String>,
    #[serde(rename = "uvIndex")]
    uv_index: Option<String>,
    astronomy: Option<Vec<WttrAstronomy>>,
    hourly: Option<Vec<WttrHourly>>,
}

#[derive(Debug, Deserialize)]
struct WttrAstronomy {
    sunrise: Option<String>,
    sunset: Option<String>,
    moonrise: Option<String>,
    moonset: Option<String>,
    moon_phase: Option<String>,
    moon_illumination: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct WttrHourly {
    time: String,
    #[serde(rename = "tempF")]
    temp_f: Option<String>,
    #[serde(rename = "tempC")]
    temp_c: Option<String>,
    #[serde(rename = "weatherDesc")]
    weather_desc: Vec<WttrValue>,
    #[serde(rename = "chanceofrain")]
    chance_of_rain: String,
    #[serde(rename = "chanceofsnow")]
    chance_of_snow: Option<String>,
    humidity: Option<String>,
    #[serde(rename = "windspeedKmph")]
    windspeed_kmph: Option<String>,
    #[serde(rename = "WindGustKmph")]
    wind_gust_kmph: Option<String>,
    cloudcover: Option<String>,
}

impl WeatherCollector {
    pub fn new(config: WeatherConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("Pulse/0.1.0")
            .build()
            .unwrap_or_default();
        Self { config, client }
    }
}

#[async_trait]
impl Collector for WeatherCollector {
    fn id(&self) -> &str {
        "weather"
    }
    fn name(&self) -> &str {
        "Weather"
    }
    fn default_interval(&self) -> Duration {
        parse_interval(&self.config.interval)
    }
    fn enabled(&self) -> bool {
        self.config.enabled
    }

    async fn collect(&self) -> Result<Vec<RawItem>> {
        let location = self.config.location.as_deref().unwrap_or("auto");
        tracing::debug!("Fetching weather for: {}", location);

        let url = format!("https://wttr.in/{}?format=j1", urlencoded(location));
        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            anyhow::bail!("wttr.in returned status {}", response.status());
        }

        let wttr: WttrResponse = response.json().await?;
        let now = Utc::now();
        let mut items = Vec::new();

        if let Some(current) = wttr.current_condition.first() {
            let desc = current
                .weather_desc
                .first()
                .map(|v| v.value.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            let area_name = wttr
                .nearest_area
                .as_ref()
                .and_then(|a| a.first())
                .and_then(|a| a.area_name.first())
                .map(|v| v.value.clone())
                .unwrap_or_else(|| location.to_string());

            let region = wttr
                .nearest_area
                .as_ref()
                .and_then(|a| a.first())
                .and_then(|a| a.region.first())
                .map(|v| v.value.clone())
                .unwrap_or_default();

            let title = format!(
                "{}: {}°F ({}°C) — {}",
                area_name, current.temp_F, current.temp_C, desc
            );

            // Build forecast with hourly data
            let forecast: Vec<serde_json::Value> = wttr
                .weather
                .unwrap_or_default()
                .into_iter()
                .map(|day| {
                    let mid_desc = day
                        .hourly
                        .as_ref()
                        .and_then(|h| h.get(4))
                        .and_then(|h| h.weather_desc.first())
                        .map(|v| v.value.clone())
                        .unwrap_or_default();
                    let rain = day
                        .hourly
                        .as_ref()
                        .and_then(|h| h.get(4))
                        .map(|h| h.chance_of_rain.clone())
                        .unwrap_or_default();

                    let hourly: Vec<serde_json::Value> = day
                        .hourly
                        .unwrap_or_default()
                        .into_iter()
                        .map(|h| {
                            let hdesc = h
                                .weather_desc
                                .first()
                                .map(|v| v.value.clone())
                                .unwrap_or_default();
                            serde_json::json!({
                                "time": format!("{}:00", h.time.parse::<u32>().unwrap_or(0) / 100),
                                "temp_f": h.temp_f,
                                "temp_c": h.temp_c,
                                "description": hdesc,
                                "rain_chance": h.chance_of_rain,
                                "humidity": h.humidity,
                                "wind_kmph": h.windspeed_kmph,
                                "cloud_cover": h.cloudcover,
                            })
                        })
                        .collect();

                    let astro = day.astronomy.as_ref().and_then(|a| a.first());

                    serde_json::json!({
                        "date": day.date,
                        "high_f": day.max_temp_f,
                        "low_f": day.min_temp_f,
                        "high_c": day.max_temp_c,
                        "low_c": day.min_temp_c,
                        "avg_f": day.avg_temp_f,
                        "description": mid_desc,
                        "rain_chance": rain,
                        "uv_index": day.uv_index,
                        "sun_hours": day.sun_hour,
                        "sunrise": astro.and_then(|a| a.sunrise.clone()),
                        "sunset": astro.and_then(|a| a.sunset.clone()),
                        "moon_phase": astro.and_then(|a| a.moon_phase.clone()),
                        "hourly": hourly,
                    })
                })
                .collect();

            let metadata = serde_json::json!({
                "location": area_name,
                "region": region,
                "temp_f": current.temp_F.parse::<f64>().unwrap_or(0.0),
                "temp_c": current.temp_C.parse::<f64>().unwrap_or(0.0),
                "feels_like_f": current.feels_like_f.parse::<f64>().unwrap_or(0.0),
                "feels_like_c": current.feels_like_c.parse::<f64>().unwrap_or(0.0),
                "humidity": current.humidity.parse::<f64>().unwrap_or(0.0),
                "description": desc,
                "wind_speed_kmph": current.windspeed_kmph.parse::<f64>().unwrap_or(0.0),
                "wind_speed_mph": current.windspeed_miles.as_ref().and_then(|s| s.parse::<f64>().ok()),
                "wind_direction": current.wind_dir,
                "wind_degree": current.winddir_degree,
                "visibility_km": current.visibility,
                "visibility_miles": current.visibility_miles,
                "uv_index": current.uv_index.parse::<f64>().unwrap_or(0.0),
                "pressure_mb": current.pressure,
                "pressure_in": current.pressure_inches,
                "cloud_cover": current.cloudcover,
                "precip_mm": current.precip_mm,
                "precip_in": current.precip_inches,
                "forecast": forecast,
            });

            items.push(RawItem {
                source: "weather".to_string(),
                collector_id: "weather".to_string(),
                title,
                url: Some(format!("https://wttr.in/{}", urlencoded(location))),
                content: Some(format!(
                    "Feels like {}°F. Humidity: {}%. Wind: {} km/h {}. Pressure: {} mb. Cloud cover: {}%.",
                    current.feels_like_f,
                    current.humidity,
                    current.windspeed_kmph,
                    current.wind_dir,
                    current.pressure.as_deref().unwrap_or("?"),
                    current.cloudcover.as_deref().unwrap_or("?")
                )),
                metadata,
                published_at: Some(now),
            });
        }

        tracing::info!("Fetched weather for {}", location);
        Ok(items)
    }
}

fn urlencoded(s: &str) -> String {
    s.replace(' ', "+")
}
