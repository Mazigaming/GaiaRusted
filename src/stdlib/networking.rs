
use std::net::{TcpListener, TcpStream, UdpSocket, SocketAddr};
use std::io::{Read, Write, Result as IoResult};

pub struct TcpServer {
    listener: TcpListener,
    addr: SocketAddr,
}

pub struct TcpClient {
    stream: TcpStream,
}

pub struct UdpEndpoint {
    socket: UdpSocket,
}

impl TcpServer {
    pub fn bind(addr: &str) -> IoResult<Self> {
        let listener = TcpListener::bind(addr)?;
        let addr: SocketAddr = addr.parse().unwrap_or_else(|_| {
            "127.0.0.1:0"
                .parse()
                .unwrap()
        });

        Ok(TcpServer { listener, addr })
    }

    pub fn accept(&self) -> IoResult<(TcpStream, SocketAddr)> {
        self.listener.accept()
    }

    pub fn local_addr(&self) -> IoResult<SocketAddr> {
        self.listener.local_addr()
    }

    pub fn set_nonblocking(&self, nonblocking: bool) -> IoResult<()> {
        self.listener.set_nonblocking(nonblocking)
    }
}

impl TcpClient {
    pub fn connect(addr: &str) -> IoResult<Self> {
        let stream = TcpStream::connect(addr)?;
        Ok(TcpClient { stream })
    }

    pub fn write(&mut self, data: &[u8]) -> IoResult<usize> {
        self.stream.write(data)
    }

    pub fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.stream.read(buf)
    }

    pub fn read_to_string(&mut self) -> IoResult<String> {
        let mut contents = String::new();
        let _ = self.stream.read_to_string(&mut contents)?;
        Ok(contents)
    }

    pub fn shutdown(&mut self) -> IoResult<()> {
        use std::net::Shutdown;
        self.stream.shutdown(Shutdown::Both)
    }

    pub fn peer_addr(&self) -> IoResult<SocketAddr> {
        self.stream.peer_addr()
    }

    pub fn local_addr(&self) -> IoResult<SocketAddr> {
        self.stream.local_addr()
    }
}

impl UdpEndpoint {
    pub fn bind(addr: &str) -> IoResult<Self> {
        let socket = UdpSocket::bind(addr)?;
        Ok(UdpEndpoint { socket })
    }

    pub fn send_to(&self, buf: &[u8], addr: &str) -> IoResult<usize> {
        self.socket.send_to(buf, addr)
    }

    pub fn recv_from(&self, buf: &mut [u8]) -> IoResult<(usize, SocketAddr)> {
        self.socket.recv_from(buf)
    }

    pub fn recv(&self, buf: &mut [u8]) -> IoResult<usize> {
        self.socket.recv(buf)
    }

    pub fn send(&self, buf: &[u8]) -> IoResult<usize> {
        self.socket.send(buf)
    }

    pub fn set_read_timeout(&self, timeout: Option<std::time::Duration>) -> IoResult<()> {
        self.socket.set_read_timeout(timeout)
    }

    pub fn set_write_timeout(&self, timeout: Option<std::time::Duration>) -> IoResult<()> {
        self.socket.set_write_timeout(timeout)
    }

    pub fn local_addr(&self) -> IoResult<SocketAddr> {
        self.socket.local_addr()
    }
}

pub struct HttpRequest {
    method: String,
    path: String,
    version: String,
    headers: std::collections::HashMap<String, String>,
    body: String,
}

impl HttpRequest {
    pub fn new(method: &str, path: &str) -> Self {
        HttpRequest {
            method: method.to_string(),
            path: path.to_string(),
            version: "HTTP/1.1".to_string(),
            headers: std::collections::HashMap::new(),
            body: String::new(),
        }
    }

    pub fn add_header(&mut self, key: &str, value: &str) {
        self.headers.insert(key.to_string(), value.to_string());
    }

    pub fn set_body(&mut self, body: &str) {
        self.body = body.to_string();
    }

    pub fn to_string(&self) -> String {
        let mut request = format!("{} {} {}\r\n", self.method, self.path, self.version);
        for (key, value) in &self.headers {
            request.push_str(&format!("{}: {}\r\n", key, value));
        }
        request.push_str("Content-Length: ");
        request.push_str(&self.body.len().to_string());
        request.push_str("\r\n\r\n");
        request.push_str(&self.body);
        request
    }
}

pub struct HttpResponse {
    status_code: u16,
    status_message: String,
    headers: std::collections::HashMap<String, String>,
    body: String,
}

impl HttpResponse {
    pub fn new(status_code: u16) -> Self {
        let status_message = match status_code {
            200 => "OK",
            201 => "Created",
            400 => "Bad Request",
            404 => "Not Found",
            500 => "Internal Server Error",
            _ => "Unknown",
        }.to_string();

        HttpResponse {
            status_code,
            status_message,
            headers: std::collections::HashMap::new(),
            body: String::new(),
        }
    }

    pub fn add_header(&mut self, key: &str, value: &str) {
        self.headers.insert(key.to_string(), value.to_string());
    }

    pub fn set_body(&mut self, body: &str) {
        self.body = body.to_string();
    }

    pub fn to_string(&self) -> String {
        let mut response = format!("HTTP/1.1 {} {}\r\n", self.status_code, self.status_message);
        for (key, value) in &self.headers {
            response.push_str(&format!("{}: {}\r\n", key, value));
        }
        response.push_str("Content-Length: ");
        response.push_str(&self.body.len().to_string());
        response.push_str("\r\n\r\n");
        response.push_str(&self.body);
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_request_creation() {
        let req = HttpRequest::new("GET", "/");
        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/");
    }

    #[test]
    fn test_http_request_with_headers() {
        let mut req = HttpRequest::new("POST", "/api");
        req.add_header("Content-Type", "application/json");
        assert!(req.to_string().contains("Content-Type"));
    }

    #[test]
    fn test_http_response_creation() {
        let resp = HttpResponse::new(200);
        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.status_message, "OK");
    }

    #[test]
    fn test_http_response_with_body() {
        let mut resp = HttpResponse::new(200);
        resp.set_body("Hello World");
        let resp_str = resp.to_string();
        assert!(resp_str.contains("Hello World"));
    }

    #[test]
    fn test_http_404_response() {
        let resp = HttpResponse::new(404);
        assert_eq!(resp.status_code, 404);
        assert_eq!(resp.status_message, "Not Found");
    }
}
