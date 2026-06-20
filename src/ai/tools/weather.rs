use adk_rust::tool::FunctionTool;
use adk_rust::{AdkError, Result as AdkResult, Tool, ToolContext};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

const GEOCODING_URL: &str = "https://geocoding-api.open-meteo.com/v1/search";
const WEATHER_URL: &str = "https://api.open-meteo.com/v1/forecast";

/// Arguments the model passes when calling the weather tool.
#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct WeatherArgs {
    /// The city to look up weather for.
    pub city: String,
}

/// Parsed geocoding response — the first match.
#[derive(Debug, PartialEq)]
pub struct GeoLocation {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub country: String,
}

/// Parsed weather response.
#[derive(Debug, PartialEq)]
pub struct WeatherData {
    pub temperature_c: f64,
    pub humidity: i64,
    pub weather_code: i64,
    pub wind_kmh: f64,
    pub condition: String,
}

/// Final user-facing result.
#[derive(Debug, PartialEq)]
pub struct WeatherResult {
    pub city: String,
    pub country: String,
    pub temperature_c: f64,
    pub humidity: i64,
    pub wind_kmh: f64,
    pub condition: String,
}

impl WeatherResult {
    pub fn to_json(&self) -> Value {
        json!({
            "city": self.city,
            "country": self.country,
            "temperature_c": self.temperature_c,
            "humidity": self.humidity,
            "wind_kmh": self.wind_kmh,
            "condition": self.condition,
        })
    }
}

/// Map a WMO weather interpretation code to a human-readable description.
/// Reference: https://open-meteo.com/en/docs (WMO Weather interpretation codes)
pub fn wmo_description(code: i64) -> &'static str {
    match code {
        0 => "clear sky",
        1 => "mainly clear",
        2 => "partly cloudy",
        3 => "overcast",
        45 => "fog",
        48 => "depositing rime fog",
        51 => "light drizzle",
        53 => "moderate drizzle",
        55 => "dense drizzle",
        56 => "light freezing drizzle",
        57 => "dense freezing drizzle",
        61 => "slight rain",
        63 => "moderate rain",
        65 => "heavy rain",
        66 => "light freezing rain",
        67 => "heavy freezing rain",
        71 => "slight snow",
        73 => "moderate snow",
        75 => "heavy snow",
        77 => "snow grains",
        80 => "slight rain showers",
        81 => "moderate rain showers",
        82 => "violent rain showers",
        85 => "slight snow showers",
        86 => "heavy snow showers",
        95 => "thunderstorm",
        96 => "thunderstorm with slight hail",
        99 => "thunderstorm with heavy hail",
        _ => "unknown",
    }
}

/// Parse the geocoding API response. Returns None if no results.
pub fn parse_geocoding(body: &str) -> std::result::Result<Option<GeoLocation>, serde_json::Error> {
    #[derive(Deserialize)]
    struct Resp {
        results: Option<Vec<GeoRaw>>,
    }
    #[derive(Deserialize)]
    struct GeoRaw {
        name: String,
        latitude: f64,
        longitude: f64,
        country: Option<String>,
    }
    let r: Resp = serde_json::from_str(body)?;
    Ok(r.results.and_then(|mut v| {
        v.drain(..).next().map(|g| GeoLocation {
            name: g.name,
            latitude: g.latitude,
            longitude: g.longitude,
            country: g.country.unwrap_or_default(),
        })
    }))
}

/// Parse the weather API response.
pub fn parse_weather(body: &str) -> std::result::Result<WeatherData, serde_json::Error> {
    #[derive(Deserialize)]
    struct Resp {
        current: Current,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "snake_case")]
    struct Current {
        temperature_2m: f64,
        relative_humidity_2m: i64,
        weather_code: i64,
        wind_speed_10m: f64,
    }
    let r: Resp = serde_json::from_str(body)?;
    Ok(WeatherData {
        temperature_c: r.current.temperature_2m,
        humidity: r.current.relative_humidity_2m,
        weather_code: r.current.weather_code,
        wind_kmh: r.current.wind_speed_10m,
        condition: wmo_description(r.current.weather_code).into(),
    })
}

/// The tool handler: geocode the city, fetch weather, return JSON.
async fn get_weather(_ctx: Arc<dyn ToolContext>, args: Value) -> AdkResult<Value> {
    let city = args["city"]
        .as_str()
        .ok_or_else(|| AdkError::tool("missing 'city' argument"))?;

    // 1. Geocode
    let geo_resp = reqwest::Client::new()
        .get(GEOCODING_URL)
        .query(&[
            ("name", city),
            ("count", "1"),
            ("language", "en"),
            ("format", "json"),
        ])
        .send()
        .await
        .map_err(|e| AdkError::tool(format!("geocoding request failed: {e}")))?;

    let geo_body = geo_resp
        .text()
        .await
        .map_err(|e| AdkError::tool(format!("geocoding read failed: {e}")))?;

    let geo = parse_geocoding(&geo_body)
        .map_err(|e| AdkError::tool(format!("geocoding parse failed: {e}")))?
        .ok_or_else(|| AdkError::tool(format!("city not found: {city}")))?;

    // 2. Fetch weather
    let w_resp = reqwest::Client::new()
        .get(WEATHER_URL)
        .query(&[
            ("latitude", geo.latitude.to_string().as_str()),
            ("longitude", geo.longitude.to_string().as_str()),
            (
                "current",
                "temperature_2m,relative_humidity_2m,weather_code,wind_speed_10m",
            ),
        ])
        .send()
        .await
        .map_err(|e| AdkError::tool(format!("weather request failed: {e}")))?;

    let w_body = w_resp
        .text()
        .await
        .map_err(|e| AdkError::tool(format!("weather read failed: {e}")))?;

    let data =
        parse_weather(&w_body).map_err(|e| AdkError::tool(format!("weather parse failed: {e}")))?;

    let result = WeatherResult {
        city: geo.name,
        country: geo.country,
        temperature_c: data.temperature_c,
        humidity: data.humidity,
        wind_kmh: data.wind_kmh,
        condition: data.condition,
    };

    Ok(result.to_json())
}

/// Build the weather tool, ready to attach to an `LlmAgentBuilder`.
pub fn build_weather_tool() -> Arc<dyn Tool> {
    Arc::new(
        FunctionTool::new(
            "get_weather",
            "Get the current weather for a city. Returns temperature (Celsius), humidity, wind speed, and conditions.",
            get_weather,
        )
        .with_parameters_schema::<WeatherArgs>(),
    )
}
