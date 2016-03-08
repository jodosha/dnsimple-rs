extern crate hyper;

use BASE_URL;

use std::io::Read;
use std::str;

pub struct Client {
    client:  hyper::Client,
    api_key: String,
}

pub struct Response {
    raw:  hyper::client::response::Response,
    body: String,
}

impl Client {
    pub fn new(api_key: &str) -> Client {
        Client {
            client:  hyper::Client::new(),
            api_key: api_key.into(),
        }
    }

    pub fn with_client(api_key: &str, client: hyper::Client) -> Client {
        Client {
            client: client,
            api_key:    api_key.into(),
        }
    }

    pub fn whoami(&self) -> Response {
        self.get("/whoami")
    }

    fn get(&self, endpoint: &'static str) -> Response {
        self.request(hyper::method::Method::Get, endpoint)
    }

    fn request(&self, method: hyper::method::Method, endpoint: &'static str) -> Response {
        let url = self.url(endpoint);

        let mut response = self.client.request(method, &url)
            .headers(self.headers())
            .send().unwrap();

        let mut body = String::new();
        response.read_to_string(&mut body).unwrap();

        Response {
            raw:  response,
            body: body
        }
    }

    fn url(&self, endpoint: &'static str) -> String {
        format!("{}/{}", BASE_URL, endpoint)
    }

    fn headers(&self) -> hyper::header::Headers {
        let mut result = hyper::header::Headers::new();

        result.set(
            hyper::header::Authorization(
                hyper::header::Bearer { token: self.api_key.clone() }
            )
        );

        result.set(hyper::header::ContentType::json());
        result
    }
}

impl Response {
    pub fn status(&self) -> u16 {
        self.raw.status.to_u16()
    }

    pub fn header(&self, name: &'static str) -> Option<&str> {
        self.raw.headers.get_raw(name)
            .and_then(|vals| str::from_utf8(&vals[0]).ok())
    }

    pub fn rate_limit(&self) -> Option<u16> {
        match self.header("X-RateLimit-Limit") {
            Some(val) => Some(val.parse::<u16>().unwrap()),
            _         => None
        }
    }

    pub fn rate_limit_remaining(&self) -> Option<u16> {
        match self.header("X-RateLimit-Remaining") {
            Some(val) => Some(val.parse::<u16>().unwrap()),
            _         => None
        }
    }

    pub fn rate_limit_reset(&self) -> Option<u32> {
        match self.header("X-RateLimit-Reset") {
            Some(val) => Some(val.parse::<u32>().unwrap()),
            _         => None
        }
    }
}

#[cfg(test)]
mod tests {
    use dnsimple::*;
    use hyper;

    use std::env;
    use std::error::Error;
    use std::fs::File;
    use std::io::prelude::*;
    use std::path::Path;
    use std::path::PathBuf;
    use std::io::BufReader;

    fn http_response_fixture(path: &'static str) -> String {
        let mut full_path = PathBuf::new();
        let root = env::current_dir().unwrap();
        full_path.push(root);
        full_path.push("fixtures.http");
        full_path.push(path);

        let path    = Path::new(&full_path);
        let display = path.display();

        let file = match File::open(&path) {
            Err(why) => panic!("couldn't open {}: {}", display, Error::description(&why)),
            Ok(file) => file,
        };

        let buf = BufReader::new(file);
        let s = buf.lines().map(|l| l.expect("Could not parse line") + "\r\n").collect();

        return s
    }

    ///////////////////////////////////////////////////////////////////////////
    // GENERIC HTTP TESTS                                                     /
    ///////////////////////////////////////////////////////////////////////////

    mock_connector!(MockSuccessHeaders {
        "https://api.dnsimple.com" => http_response_fixture("whoami/success.http")
    });

    #[test]
    fn test_response_headers() {
        let c = hyper::client::Client::with_connector(MockSuccessHeaders::default());

        let client = Client::with_client("abc123", c);
        let response = client.whoami();

        assert_eq!(response.status(), 200);

        assert_eq!(response.header("Unknown"),                   None);
        assert_eq!(response.header("Content-Type"),              Some("application/json; charset=utf-8"));
        assert_eq!(response.header("Cache-Control"),             Some("max-age=0, private, must-revalidate"));
        assert_eq!(response.header("X-Request-Id"),              Some("15a7f3a5-7ee5-4e36-ac5a-8c21c2e1fffd"));
        assert_eq!(response.header("Date"),                      Some("Fri, 18 Dec 2015 15:19:37 GMT"));
        assert_eq!(response.header("Strict-Transport-Security"), Some("max-age=31536000"));
        assert_eq!(response.header("X-RateLimit-Limit"),         Some("4000"));
        assert_eq!(response.header("X-RateLimit-Remaining"),     Some("3991"));
        assert_eq!(response.header("X-RateLimit-Reset"),         Some("1450451976"));
        assert_eq!(response.header("ETag"),                      Some("W/\"5ea6326bc1a8e83e5c156c564f2559f0\""));
    }

    #[test]
    fn test_response_rate_limit() {
        let c = hyper::client::Client::with_connector(MockSuccessHeaders::default());

        let client = Client::with_client("abc123", c);
        let response = client.whoami();

        assert_eq!(response.rate_limit(), Some(4000));
    }

    #[test]
    fn test_response_rate_limit_remaining() {
        let c = hyper::client::Client::with_connector(MockSuccessHeaders::default());

        let client = Client::with_client("abc123", c);
        let response = client.whoami();

        assert_eq!(response.rate_limit_remaining(), Some(3991));
    }

    #[test]
    fn test_response_rate_limit_reset() {
        let c = hyper::client::Client::with_connector(MockSuccessHeaders::default());

        let client = Client::with_client("abc123", c);
        let response = client.whoami();

        assert_eq!(response.rate_limit_reset(), Some(1450451976));
    }

    mock_connector!(MockUnauthorizedResponse {
        "https://api.dnsimple.com" => http_response_fixture("whoami/unauthorized.http")
    });

    #[test]
    fn test_unauthorized() {
        let c = hyper::client::Client::with_connector(MockUnauthorizedResponse::default());

        let client = Client::with_client("wrong-api-key", c);
        let response = client.whoami();

        assert_eq!(response.status(), 401);
        assert_eq!(response.header("Content-Type"),  Some("application/json; charset=utf-8"));
        assert_eq!(response.header("Cache-Control"), Some("no-cache"));

        assert_eq!(response.rate_limit(),           None);
        assert_eq!(response.rate_limit_remaining(), None);
        assert_eq!(response.rate_limit_reset(),     None);

        assert_eq!(response.body, "{\"message\":\"Authentication failed\"}\r\n");
    }

    mock_connector!(MockErrorResponse {
        "https://api.dnsimple.com" => http_response_fixture("whoami/internal_server_error.http")
    });

    #[test]
    fn test_server_side_error() {
        let c = hyper::client::Client::with_connector(MockErrorResponse::default());

        let client = Client::with_client("abc123", c);
        let response = client.whoami();

        assert_eq!(response.status(), 500);
        assert_eq!(response.header("Content-Type"),  Some("application/json; charset=utf-8"));
        assert_eq!(response.header("Cache-Control"), Some("no-cache"));

        assert_eq!(response.rate_limit(),           None);
        assert_eq!(response.rate_limit_remaining(), None);
        assert_eq!(response.rate_limit_reset(),     None);

        assert_eq!(response.body, "{\"message\":\"Internal Server Error\"}\r\n");
    }
}
