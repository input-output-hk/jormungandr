use crate::common::jormungandr::{ConfigurationBuilder, Starter};
use assert_fs::TempDir;
use jormungandr_lib::interfaces::{Cors, CorsOrigin};
use jormungandr_testing_utils::testing::node::JormungandrRest;

#[test]
pub fn cors_illegal_domain() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new().unwrap();

    let config = ConfigurationBuilder::new()
        .with_rest_cors_config(Cors {
            allowed_origins: vec!["http://domain.com".to_owned().into()],
            max_age_secs: None,
        })
        .build(&temp_dir);

    let jormungandr = Starter::new().config(config).start().unwrap();

    let mut rest_client = jormungandr.rest();
    rest_client.set_origin("http://other_domain.com");

    assert_request_failed_due_to_cors(&rest_client)?;
    Ok(())
}

fn assert_request_failed_due_to_cors(
    rest_client: &JormungandrRest,
) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        rest_client.stats_raw()?.text()?,
        "CORS request forbidden: origin not allowed"
    );
    Ok(())
}

#[test]
pub fn cors_malformed_domain_no_http() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new().unwrap();
    let config = ConfigurationBuilder::new()
        .with_rest_cors_config(Cors {
            allowed_origins: vec!["domain.com".to_owned().into()],
            max_age_secs: None,
        })
        .build(&temp_dir);

    let jormungandr = Starter::new().config(config).start_async().unwrap();

    let mut rest_client = jormungandr.rest();
    rest_client.set_origin("domain.com");

    assert!(
        rest_client.stats_raw().is_err(),
        "jormungandr should not bootstrap with incorrect cors setting"
    );
    Ok(())
}

#[test]
pub fn cors_ip_versus_domain() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new().unwrap();

    let config = ConfigurationBuilder::new()
        .with_rest_cors_config(Cors {
            allowed_origins: vec!["http://127.0.0.1".to_owned().into()],
            max_age_secs: None,
        })
        .build(&temp_dir);

    let jormungandr = Starter::new().config(config).start_async().unwrap();

    let mut rest_client = jormungandr.rest();
    rest_client.set_origin("http://localhost");

    println!("{:?}", rest_client.stats_raw()?.status());
    Ok(())
}

#[test]
pub fn cors_wrong_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new().unwrap();

    let config = ConfigurationBuilder::new()
        .with_rest_cors_config(Cors {
            allowed_origins: vec!["http://domain.com,http://other_domain.com"
                .to_owned()
                .into()],
            max_age_secs: None,
        })
        .build(&temp_dir);

    Starter::new()
        .config(config)
        .start_fail("rest.cors.allowed_origins[0]: invalid value");
    Ok(())
}

#[test]
pub fn cors_single_domain() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new().unwrap();

    let config = ConfigurationBuilder::new()
        .with_rest_cors_config(Cors {
            allowed_origins: vec!["http://domain.com".to_owned().into()],
            max_age_secs: None,
        })
        .build(&temp_dir);

    let jormungandr = Starter::new().config(config).start_async().unwrap();

    let mut rest_client = jormungandr.rest();
    rest_client.set_origin("http://domain.com");

    assert!(rest_client.stats_raw()?.status().is_success());

    Ok(())
}

#[test]
pub fn cors_https() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new().unwrap();

    let config = ConfigurationBuilder::new()
        .with_rest_cors_config(Cors {
            allowed_origins: vec!["https://domain.com".to_owned().into()],
            max_age_secs: None,
        })
        .build(&temp_dir);

    let jormungandr = Starter::new().config(config).start_async().unwrap();

    let mut rest_client = jormungandr.rest();
    rest_client.set_origin("https://domain.com");

    assert!(rest_client.stats_raw()?.status().is_success());

    Ok(())
}

#[test]
pub fn cors_multi_domain() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new().unwrap().into_persistent();

    let config = ConfigurationBuilder::new()
        .with_rest_cors_config(Cors {
            allowed_origins: vec!["http://domain.com;http://other_domain.com"
                .to_owned()
                .into()],
            max_age_secs: None,
        })
        .build(&temp_dir);

    Starter::new()
        .config(config)
        .start_fail("invalid value: string \"http://domain.com;http://other_domain.com\"");

    Ok(())
}
