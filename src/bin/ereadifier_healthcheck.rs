use ereadifier::config::load_config;

#[derive(Debug)]
#[allow(dead_code)] // enum internal values are only used by Debug impl, so dead code analysis considers them unused
enum Error {
    UreqError(ureq::Error),
    HealthCheckError(String),
}

impl Error {
    fn from_ureq(err: ureq::Error) -> Self {
        Self::UreqError(err)
    }

    fn from_str(err: impl AsRef<str>) -> Self {
        Self::HealthCheckError(err.as_ref().to_owned())
    }
}

fn main() -> Result<(), Error> {
    let config = load_config();

    let port = config.listen_on.port();
    let localhost_with_port =
        std::net::SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);

    println!("Checking response from http://{localhost_with_port}/health ...");

    let body = ureq::get(format!("http://{localhost_with_port}/health"))
        .call()
        .map_err(Error::from_ureq)?
        .body_mut()
        .read_to_string()
        .map_err(Error::from_ureq)?;

    match body.as_str() {
        "OK" => {
            println!("Healthy - got 'OK'");
            Ok(())
        }
        s => Err(Error::from_str(format!("expected 'OK', got '{s}'"))),
    }
}
