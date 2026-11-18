use hermes_native::app::HermesNativeApp;
use hermes_native::native_window::ReadinessTarget;

#[derive(Debug, PartialEq, Eq)]
struct WindowLaunch {
    id: u64,
    route: String,
    profile: Option<String>,
    readiness_target: ReadinessTarget,
}

fn parse_window_launch() -> Result<Option<WindowLaunch>, String> {
    parse_window_launch_args(std::env::args().skip(1))
}

fn parse_window_launch_args(
    values: impl IntoIterator<Item = String>,
) -> Result<Option<WindowLaunch>, String> {
    let mut args = values.into_iter();
    let Some(mode) = args.next() else {
        return Ok(None);
    };
    if mode != "--hermes-native-window" {
        return Err(format!("Unexpected Hermes argument: {mode}"));
    }
    let id = args
        .next()
        .ok_or_else(|| "Native window ID is missing".to_owned())?
        .parse::<u64>()
        .map_err(|_| "Native window ID must be an unsigned integer".to_owned())?;
    if id == 0 {
        return Err("Native window ID must be greater than zero".into());
    }
    if args.next().as_deref() != Some("--route") {
        return Err("Native window route argument is missing".into());
    }
    let route = args
        .next()
        .ok_or_else(|| "Native window route is missing".to_owned())?;
    if route.len() > 2048 || route.chars().any(char::is_control) {
        return Err("Native window route is invalid or exceeds 2048 bytes".into());
    }
    let mut profile = None;
    let mut ready_address = None;
    let mut ready_nonce = None;
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--profile" if profile.is_none() => {
                let value = args
                    .next()
                    .ok_or_else(|| "Native window profile is missing".to_owned())?;
                if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
                    return Err("Native window profile is invalid".into());
                }
                profile = Some(value);
            }
            "--ready-address" if ready_address.is_none() => {
                ready_address = Some(
                    args.next()
                        .ok_or_else(|| "Native window readiness address is missing".to_owned())?,
                );
            }
            "--ready-nonce" if ready_nonce.is_none() => {
                ready_nonce = Some(
                    args.next()
                        .ok_or_else(|| "Native window readiness nonce is missing".to_owned())?,
                );
            }
            _ => return Err(format!("Unexpected or duplicate Hermes argument: {flag}")),
        }
    }
    let ready_address =
        ready_address.ok_or_else(|| "Native window readiness address is missing".to_owned())?;
    let ready_nonce =
        ready_nonce.ok_or_else(|| "Native window readiness nonce is missing".to_owned())?;
    let readiness_target = ReadinessTarget::parse(&ready_address, &ready_nonce)?;
    Ok(Some(WindowLaunch {
        id,
        route,
        profile,
        readiness_target,
    }))
}

fn main() -> eframe::Result {
    let window = match parse_window_launch() {
        Ok(window) => window,
        Err(error) => {
            eprintln!("Hermes window startup failed: {error}");
            std::process::exit(2);
        }
    };
    let title = window.as_ref().map_or_else(
        || "Hermes".to_owned(),
        |window| format!("Hermes · Window {}", window.id),
    );
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(title.clone())
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([420.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        &title,
        options,
        Box::new(|cc| {
            let app = match &window {
                Some(window) => HermesNativeApp::new_for_native_window(
                    cc,
                    window.id,
                    &window.route,
                    window.profile.as_deref(),
                    window.readiness_target.clone(),
                ),
                None => HermesNativeApp::new(cc),
            };
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_launch_arguments_preserve_route_and_profile() {
        let launch = parse_window_launch_args(
            [
                "--hermes-native-window",
                "73",
                "--route",
                "/capabilities?tab=plugins",
                "--profile",
                "work",
                "--ready-address",
                "127.0.0.1:1234",
                "--ready-nonce",
                &"ab".repeat(32),
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .unwrap();
        assert_eq!(
            launch,
            Some(WindowLaunch {
                id: 73,
                route: "/capabilities?tab=plugins".into(),
                profile: Some("work".into()),
                readiness_target: ReadinessTarget::parse("127.0.0.1:1234", &"ab".repeat(32),)
                    .unwrap(),
            })
        );
    }

    #[test]
    fn window_launch_arguments_allow_missing_profile_when_receipt_is_present() {
        let launch = parse_window_launch_args(
            [
                "--hermes-native-window".to_owned(),
                "74".to_owned(),
                "--route".to_owned(),
                "/".to_owned(),
                "--ready-address".to_owned(),
                "127.0.0.1:1234".to_owned(),
                "--ready-nonce".to_owned(),
                "ab".repeat(32),
            ]
            .into_iter(),
        )
        .expect("parse native window command line");
        assert_eq!(
            launch,
            Some(WindowLaunch {
                id: 74,
                route: "/".into(),
                profile: None,
                readiness_target: ReadinessTarget::parse("127.0.0.1:1234", &"ab".repeat(32),)
                    .unwrap(),
            })
        );
    }

    #[test]
    fn window_launch_arguments_reject_invalid_identity_route_and_trailing_input() {
        for args in [
            vec!["--hermes-native-window", "0", "--route", "/"],
            vec!["--hermes-native-window", "abc", "--route", "/"],
            vec!["--hermes-native-window", "3", "--route", "/x\ny"],
            vec!["--hermes-native-window", "3", "--route", "/", "extra"],
            vec![
                "--hermes-native-window",
                "3",
                "--route",
                "/",
                "--ready-address",
                "127.0.0.1:1234",
                "--ready-nonce",
                "short",
            ],
        ] {
            assert!(parse_window_launch_args(args.into_iter().map(str::to_owned)).is_err());
        }
    }
}
