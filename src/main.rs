use std::net::SocketAddr;
use std::convert::Infallible;
use std::str;
use hyper::service::{make_service_fn, service_fn};
use hyper::header;
use hyper::{body::HttpBody as _, Client};
use hyper::{Body, Method, Request, Response, StatusCode, Server};
use hyper_tls::HttpsConnector;
use csv::Reader;

/// This is our service handler. It receives a Request, routes on its
/// path, and returns a Future of a Response.
async fn handle_request(req: Request<Body>) -> Result<Response<Body>, anyhow::Error> {
    match (req.method(), req.uri().path()) {
        // Serve some instructions at /
        (&Method::GET, "/") => Ok(Response::new(Body::from(
            "Try POSTing data to /find_rate such as: `curl localhost:8001/get_rate -XPOST -d '78701'`",
        ))),

        (&Method::POST, "/find_rate") => {
            let post_body = hyper::body::to_bytes(req.into_body()).await?;
            let mut rate = "0.08".to_string(); // default is 8%

            let rates_data: &[u8] = include_bytes!("rates_by_zipcode.csv");
            fetch_url_return_str("https://api.api-ninjas.com/v1/salestax?zip_code=90210".parse::<hyper::Uri>()?).await?;
            let mut rdr = Reader::from_reader(rates_data);
            for result in rdr.records() {
                let record = result?;
                // dbg!("{:?}", record.clone());
                if str::from_utf8(&post_body).unwrap().eq(&record[0]) {
                    rate = record[1].to_string();
                    break;
                }
            }

            Ok(Response::new(Body::from(rate)))
        }

        // Return the 404 Not Found for other routes.
        _ => {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8001));
    let make_svc = make_service_fn(|_| {
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                handle_request(req)
            }))
        }
    });
    let server = Server::bind(&addr).serve(make_svc);
    dbg!("Server started on port 8001");
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
    Ok(())
}

async fn fetch_url_return_str(url: hyper::Uri) -> Result<(), Infallible> {
    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);
    let client_request = Request::builder()
        .method(Method::GET)
        .uri(url)
        .header(header::ACCEPT, "*/*")
        .header(header::ACCEPT_LANGUAGE, "en-US,en;q=0.9")
        .header(header::ORIGIN, "https://api-ninjas.com")
        .header(header::HOST, "api.api-ninjas.com")
        .header(header::REFERER, "https://api-ninjas.com")
        .body(Body::empty())?;
    // Use request above to get URL

    let mut res = client.request(client_request).await?;

    let mut resp_data = Vec::new();
    while let Some(next) = res.data().await {
        let chunk = next?;
        resp_data.extend_from_slice(&chunk);
    }
    println!("{}", String::from_utf8_lossy(&resp_data));

    Ok(())
}