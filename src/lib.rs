pub mod config {
    use serde::Deserialize;

    pub struct Config {
        pub dimensions: Option<(u32, u32)>,
        pub lossy_quality: f32,
        pub encode_strategy: EncodeStrategy,
        pub double_page_strategy: DoublePageStrategy,
        pub listen_on: std::net::SocketAddr,
        pub debug_mode: bool,
    }

    pub enum EncodeStrategy {
        Smallest,
        Lossless,
        Lossy,
    }

    #[derive(PartialEq)]
    pub enum DoublePageStrategy {
        Ignore,
        IfWider,
        IfMuchWider,
    }

    impl std::fmt::Display for EncodeStrategy {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(match *self {
                Self::Smallest => "smallest (Compare lossless and lossy, use smallest)",
                Self::Lossless => "lossless (Always use lossless)",
                Self::Lossy => "lossy (Always use lossy)",
            })
        }
    }

    impl std::fmt::Display for DoublePageStrategy {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(match *self {
                Self::Ignore => "ignore (No special handling of double pages)",
                Self::IfWider => "wider (Assume input is a double page if wider than 1:1)",
                Self::IfMuchWider => "much_wider (Assume input is a double page if wider than 3:2)",
            })
        }
    }

    const PRESETS: &str = include_str!("../presets.toml");

    #[derive(Deserialize)]
    struct Preset {
        width: u32,
        height: u32,
        devices: Vec<String>,
    }

    fn parse_dimensions<S>(dims: S) -> Option<(u32, u32)>
    where
        S: AsRef<str>,
    {
        let dims = dims.as_ref().trim().to_lowercase();

        let dims_regex =
            regex::Regex::new(r"^([0-9]+)\s?x\s?([0-9]+)$").expect("regex should always be valid");
        if let Some(caps) = dims_regex.captures(&dims) {
            let (_, [width, height]) = caps.extract();
            match (width.parse(), height.parse()) {
                (Ok(w), Ok(h)) => Some((w, h)),
                _ => None,
            }
        } else {
            let presets: std::collections::HashMap<String, Preset> =
                toml::from_str(PRESETS).expect("presets.toml must always contain valid data");

            if let Some(preset) = presets.get(&dims) {
                eprintln!("Using preset `{dims}`, which supports:");
                for device in &preset.devices {
                    eprintln!("\t- {device}");
                }
                Some((preset.width, preset.height))
            } else {
                eprintln!(
                    "WARN: '{dims}' was not recognized as a valid WxH dimension string or preset."
                );
                eprintln!("Valid presets:");
                for (preset_name, Preset { width, height, .. }) in &presets {
                    eprintln!("\t- {preset_name} ({width}x{height})");
                }
                None
            }
        }
    }

    fn parse_encode_strategy<S>(strat: S) -> Option<EncodeStrategy>
    where
        S: AsRef<str>,
    {
        let strat = strat.as_ref().trim().to_lowercase();

        if strat == "smallest" {
            Some(EncodeStrategy::Smallest)
        } else if strat == "lossy" {
            Some(EncodeStrategy::Lossy)
        } else if strat == "lossless" {
            Some(EncodeStrategy::Lossless)
        } else if strat.is_empty() {
            None
        } else {
            eprintln!("WARN: '{strat}' was not recognized as a valid encode strategy");
            None
        }
    }

    fn parse_double_page_strategy<S>(strat: S) -> Option<DoublePageStrategy>
    where
        S: AsRef<str>,
    {
        let strat = strat.as_ref().trim().to_lowercase();

        if strat == "much_wider" {
            Some(DoublePageStrategy::IfMuchWider)
        } else if strat == "wider" {
            Some(DoublePageStrategy::IfWider)
        } else if strat == "ignore" {
            Some(DoublePageStrategy::Ignore)
        } else if strat.is_empty() {
            None
        } else {
            eprintln!("WARN: '{strat}' was not recognized as a valid double-page strategy");
            None
        }
    }

    pub fn load_config() -> Config {
        Config {
            dimensions: std::env::var("EREADIFIER_DIMENSIONS")
                .ok()
                .and_then(parse_dimensions),
            encode_strategy: std::env::var("EREADIFIER_ENCODE")
                .ok()
                .and_then(parse_encode_strategy)
                .unwrap_or(EncodeStrategy::Lossless),
            double_page_strategy: std::env::var("EREADIFIER_DOUBLE_PAGE")
                .ok()
                .and_then(parse_double_page_strategy)
                .unwrap_or(DoublePageStrategy::IfMuchWider),
            lossy_quality: std::env::var("EREADIFIER_LOSSY_QUALITY")
                .ok()
                .and_then(|q| q.parse::<f32>().ok())
                .unwrap_or(85.0)
                .clamp(0.0, 100.0),
            listen_on: std::env::var("EREADIFIER_LISTEN")
                .ok()
                .and_then(|addr| addr.parse::<std::net::SocketAddr>().ok())
                .unwrap_or(std::net::SocketAddr::new(
                    std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
                    80,
                )),
            debug_mode: std::env::var("EREADIFIER_DEBUG")
                .is_ok_and(|d| matches!(d.trim().to_lowercase().as_str(), "true" | "yes" | "1")),
        }
    }

    pub fn log_config(config: &Config) {
        eprintln!("Listening on {} with config:", config.listen_on);
        eprintln!(
            "\tDimensions: {}",
            config
                .dimensions
                .map_or("None (no resize)".into(), |(w, h)| format!("{w} x {h}"))
        );
        eprintln!("\tLossy quality target (0~100): {}", config.lossy_quality);
        eprintln!("\tEncoding strategy: {}", config.encode_strategy);
        eprintln!("\tDouble page strategy: {}", config.double_page_strategy);
    }
}
