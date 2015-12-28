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

    mock_connector!(MockSuccessHeaders {
        "https://api.dnsimple.com" => "HTTP/1.1 200 OK\r\n\
                                       Server: nginx\r\n\
                                       Date: Mon, 28 Dec 2015 11:15:18 GMT\r\n\
                                       Content-Type: application/json; charset=utf-8\r\n\
                                       Connection: keep-alive\r\n\
                                       Status: 200 OK\r\n\
                                       X-RateLimit-Limit: 4000\r\n\
                                       X-RateLimit-Remaining: 3999\r\n\
                                       X-RateLimit-Reset: 1451301317\r\n\
                                       ETag: W/\"8ddafbf56b04655d328ab078abe2e08d\"\r\n\
                                       Cache-Control: max-age=0, private, must-revalidate\r\n\
                                       X-Request-Id: 2bbc52dd-df8e-4a2a-9266-9979cb4cf30a\r\n\
                                       X-Runtime: 0.016579\r\n\
                                       Strict-Transport-Security: max-age=31536000\r\n\r\n"
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
        assert_eq!(response.header("X-Request-Id"),              Some("2bbc52dd-df8e-4a2a-9266-9979cb4cf30a"));
        assert_eq!(response.header("Date"),                      Some("Mon, 28 Dec 2015 11:15:18 GMT"));
        assert_eq!(response.header("Strict-Transport-Security"), Some("max-age=31536000"));
        assert_eq!(response.header("X-RateLimit-Limit"),         Some("4000"));
        assert_eq!(response.header("X-RateLimit-Remaining"),     Some("3999"));
        assert_eq!(response.header("X-RateLimit-Reset"),         Some("1451301317"));
        assert_eq!(response.header("ETag"),                      Some("W/\"8ddafbf56b04655d328ab078abe2e08d\""));
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

        assert_eq!(response.rate_limit_remaining(), Some(3999));
    }

    #[test]
    fn test_response_rate_limit_reset() {
        let c = hyper::client::Client::with_connector(MockSuccessHeaders::default());

        let client = Client::with_client("abc123", c);
        let response = client.whoami();

        assert_eq!(response.rate_limit_reset(), Some(1451301317));
    }

    mock_connector!(MockUnauthorizedResponse {
        "https://api.dnsimple.com" => "HTTP/1.1 401 Unauthorized\r\n\
                                       Server: nginx\r\n\
                                       Date: Mon, 28 Dec 2015 13:44:31 GMT\r\n\
                                       Content-Type: application/json; charset=utf-8\r\n\
                                       Connection: keep-alive\r\n\
                                       Status: 401 Unauthorized\r\n\
                                       Cache-Control: no-cache\r\n\
                                       X-Request-Id: f8a21a21-dc63-4451-9bf3-99824d9d14c5\r\n\
                                       X-Runtime: 0.009355\r\n\
                                       \r\n\
                                       {\"message\":\"Authentication failed\"}"
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

        assert_eq!(response.body, "{\"message\":\"Authentication failed\"}");
    }

    mock_connector!(MockErrorResponse {
        "https://api.dnsimple.com" => "HTTP/1.1 500 Internal Server Error\r\n\
                                       Server: nginx\r\n\
                                       Date: Mon, 28 Dec 2015 14:18:07 GMT\r\n\
                                       Content-Type: application/json; charset=utf-8\r\n\
                                       Connection: keep-alive\r\n\
                                       Status: 500 Internal Server Error\r\n\
                                       Cache-Control: no-cache\r\n\
                                       X-Request-Id: ed8acc9f-9272-4286-b6fb-0ee79b10e180\r\n\
                                       X-Runtime: 0.008398\r\n\
                                       \r\n\
                                       {\"message\":\"Internal Server Error\"}"
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

        assert_eq!(response.body, "{\"message\":\"Internal Server Error\"}");
    }
}
