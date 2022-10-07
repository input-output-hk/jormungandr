use crate::startup::SingleNodeTestBootstrapper;
use assert_fs::TempDir;
use jormungandr_automation::jormungandr::{JormungandrRest, NodeConfigBuilder, StartupError};
use jormungandr_lib::interfaces::Cors;

#[test]
pub fn cors_illegal_domain() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new().unwrap();

    let config = NodeConfigBuilder::default().with_rest_cors_config(Cors {
        allowed_origins: vec!["http://domain.com".to_string().into()],
        max_age_secs: None,
        allowed_headers: vec![],
        allowed_methods: vec![],
    });

    let jormungandr = SingleNodeTestBootstrapper::default()
        .with_node_config(config)
        .as_bft_leader()
        .build()
        .start_node(temp_dir)
        .unwrap();

    let mut rest_client = jormungandr.rest();
    rest_client.set_origin("http://other_domain.com");

    assert_request_failed_due_to_cors(&rest_client)?;
    Ok(())
}

fn assert_request_failed_due_to_cors(
    rest_client: &JormungandrRest,
) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        rest_client.raw().stats()?.text()?,
        "CORS request forbidden: origin not allowed"
    );
    Ok(())
}

#[test]
pub fn cors_malformed_domain_no_http() -> Result<(), StartupError> {
    let temp_dir = TempDir::new().unwrap();
    let config = NodeConfigBuilder::default().with_rest_cors_config(Cors {
        allowed_origins: vec!["domain.com".to_owned().into()],
        max_age_secs: None,
        allowed_headers: vec![],
        allowed_methods: vec![],
    });

    SingleNodeTestBootstrapper::default()
        .with_node_config(config)
        .as_bft_leader()
        .build()
        .starter(temp_dir)
        .unwrap()
        .start_should_fail_with_message("invalid value: string \"domain.com\"")
}

#[test]
#[cfg(windows)]
pub fn cors_ip_versus_domain() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new().unwrap();

    let config = NodeConfigBuilder::default().with_rest_cors_config(Cors {
        allowed_origins: vec!["http://127.0.0.1".to_owned().into()],
        max_age_secs: None,
        allowed_headers: vec![],
        allowed_methods: vec![],
    });

    let jormungandr = SingleNodeTestBootstrapper::default()
        .with_node_config(config)
        .as_bft_leader()
        .build()
        .start_node(temp_dir)
        .unwrap();

    let mut rest_client = jormungandr.rest();
    rest_client.set_origin("http://localhost");

    assert_eq!(rest_client.raw().stats()?.status(), 403);
    Ok(())
}

#[test]
pub fn cors_wrong_delimiter() -> Result<(), StartupError> {
    let temp_dir = TempDir::new().unwrap();

    let config = NodeConfigBuilder::default().with_rest_cors_config(Cors {
        allowed_origins: vec!["http://domain.com,http://other_domain.com"
            .to_owned()
            .into()],
        max_age_secs: None,
        allowed_headers: vec![],
        allowed_methods: vec![],
    });

    SingleNodeTestBootstrapper::default()
        .with_node_config(config)
        .as_bft_leader()
        .build()
        .starter(temp_dir)?
        .start_should_fail_with_message("rest.cors.allowed_origins[0]: invalid value")
}

#[test]
#[cfg(windows)]
pub fn cors_single_domain() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new().unwrap();

    let config = NodeConfigBuilder::default().with_rest_cors_config(Cors {
        allowed_origins: vec!["http://domain.com".to_owned().into()],
        max_age_secs: None,
        allowed_headers: vec![],
        allowed_methods: vec![],
    });

    let jormungandr = SingleNodeTestBootstrapper::default()
        .with_node_config(config)
        .as_bft_leader()
        .build()
        .start_node(temp_dir)
        .unwrap();

    let mut rest_client = jormungandr.rest();
    rest_client.set_origin("http://domain.com");

    assert!(rest_client.raw().stats()?.status().is_success());

    Ok(())
}

#[test]
#[cfg(windows)]
pub fn cors_https() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new().unwrap();

    let config = NodeConfigBuilder::default().with_rest_cors_config(Cors {
        allowed_origins: vec!["https://domain.com".to_owned().into()],
        max_age_secs: None,
        allowed_headers: vec![],
        allowed_methods: vec![],
    });

    let jormungandr = SingleNodeTestBootstrapper::default()
        .with_node_config(config)
        .as_bft_leader()
        .build()
        .starter(temp_dir)
        .unwrap()
        .start_async()
        .unwrap();

    let mut rest_client = jormungandr.rest();
    rest_client.set_origin("https://domain.com");

    assert!(rest_client.raw().stats()?.status().is_success());

    Ok(())
}

#[test]
pub fn cors_multi_domain() -> Result<(), StartupError> {
    let temp_dir = TempDir::new().unwrap();

    let config = NodeConfigBuilder::default().with_rest_cors_config(Cors {
        allowed_origins: vec!["http://domain.com;http://other_domain.com"
            .to_owned()
            .into()],
        max_age_secs: None,
        allowed_headers: vec![],
        allowed_methods: vec![],
    });

    SingleNodeTestBootstrapper::default()
        .with_node_config(config)
        .as_bft_leader()
        .build()
        .starter(temp_dir)
        .unwrap()
        .start_should_fail_with_message(
            "invalid value: string \"http://domain.com;http://other_domain.com\"",
        )
}
