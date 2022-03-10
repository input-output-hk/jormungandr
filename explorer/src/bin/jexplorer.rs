use tracing::{span, Level};

#[tokio::main]
async fn main() -> Result<(), explorer::Error> {
    let mut settings = explorer::Settings::load()?;

    let (_guards, log_init_messages) = settings.log_settings.take().unwrap().init_log()?;

    let init_span = span!(Level::TRACE, "task", kind = "init");
    let _enter = init_span.enter();
    tracing::info!("Starting explorer");

    if let Some(msgs) = log_init_messages {
        // if log settings were overriden, we will have an info
        // message which we can unpack at this point.
        for msg in &msgs {
            tracing::info!("{}", msg);
        }
    }

    let exit_status = explorer::main(settings).await;

    if let Err(error) = exit_status.as_ref() {
        tracing::error!("process finished with error: {:?}", error);

        // TODO: map to custom error code
        std::process::exit(1);
    } else {
        Ok(())
    }
}
