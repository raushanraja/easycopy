use easycopy::ai::tools::weather::{
    parse_geocoding, parse_weather, wmo_description, WeatherResult,
};

#[test]
fn wmo_clear_sky() {
    assert_eq!(wmo_description(0), "clear sky");
}

#[test]
fn wmo_partly_cloudy() {
    assert_eq!(wmo_description(2), "partly cloudy");
}

#[test]
fn wmo_rain() {
    assert_eq!(wmo_description(63), "moderate rain");
}

#[test]
fn wmo_thunderstorm() {
    assert_eq!(wmo_description(95), "thunderstorm");
}

#[test]
fn wmo_unknown_code() {
    assert_eq!(wmo_description(999), "unknown");
}

#[test]
fn parse_geocoding_extracts_lat_lon() {
    let json = r#"{
        "results": [
            {"name": "Tokyo", "latitude": 35.69, "longitude": 139.69, "country": "Japan"}
        ]
    }"#;
    let geo = parse_geocoding(json).unwrap().unwrap();
    assert_eq!(geo.name, "Tokyo");
    assert!((geo.latitude - 35.69).abs() < 0.01);
    assert!((geo.longitude - 139.69).abs() < 0.01);
    assert_eq!(geo.country, "Japan");
}

#[test]
fn parse_geocoding_no_results() {
    let json = r#"{"results": null}"#;
    assert!(parse_geocoding(json).unwrap().is_none());
}

#[test]
fn parse_weather_extracts_fields() {
    let json = r#"{
        "current": {
            "temperature_2m": 21.3,
            "relative_humidity_2m": 65,
            "weather_code": 1,
            "wind_speed_10m": 12.5
        }
    }"#;
    let w = parse_weather(json).unwrap();
    assert!((w.temperature_c - 21.3).abs() < 0.01);
    assert_eq!(w.humidity, 65);
    assert_eq!(w.weather_code, 1);
    assert!((w.wind_kmh - 12.5).abs() < 0.01);
    assert_eq!(w.condition, "mainly clear");
}

#[test]
fn weather_result_to_json_includes_city_and_temp() {
    let r = WeatherResult {
        city: "Tokyo".into(),
        country: "Japan".into(),
        temperature_c: 21.3,
        humidity: 65,
        wind_kmh: 12.5,
        condition: "mainly clear".into(),
    };
    let v = r.to_json();
    assert_eq!(v["city"], "Tokyo");
    assert_eq!(v["temperature_c"], 21.3);
    assert_eq!(v["condition"], "mainly clear");
}
