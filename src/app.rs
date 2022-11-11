use crate::request::{Method, Request};
use crate::response::Response;
use crate::status::Status;

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::Arc;

type Handler = fn(Request) -> Response;

pub type Routes = HashMap<String, Arc<Handler>>;

pub struct App<T>
where
    T: ToSocketAddrs,
{
    addr: T,
    routes: Routes,
}

impl<T> App<T>
where
    T: ToSocketAddrs,
{
    pub fn new(addr: T) -> Self {
        Self {
            addr,
            routes: HashMap::default(),
        }
    }

    pub fn add(
        &mut self,
        method: Method,
        route: &str,
        handle: Handler,
    ) -> Result<(), &str> {
        if !route.starts_with("/") {
            return Err("Route must start with /");
        }

        let method = method.to_str();
        let route = format!("{} {} HTTP/1.1", method, route);
        self.routes.insert(route, Arc::new(handle));

        Ok(())
    }

    pub fn serve(&self) -> std::io::Result<()> {
        let stream = TcpListener::bind(&self.addr)?;

        for stream in stream.incoming() {
            let stream = match stream {
                Ok(stream) => stream,
                Err(e) => panic!("{}", e),
            };
            // Thread pool
            self.handle_connection(stream)?;
        }

        Ok(())
    }

    pub fn get_routes(&self) -> &Routes {
        &self.routes
    }

    pub fn get_addr(&self) -> &T {
        &self.addr
    }

    fn respond(
        stream: &mut TcpStream,
        response: &mut Response,
    ) -> std::io::Result<()> {
        stream.write_all(response.data().as_bytes())?;
        stream.flush()?;
        Ok(())
    }

    fn handle_connection<'a>(
        &'a self,
        mut stream: TcpStream,
    ) -> std::io::Result<()> {
        let buf_reader = BufReader::new(&mut stream);
        let request_lines = BufReader::lines(buf_reader);

        let req = Request::new(request_lines);

        let handle = match self.routes.get(req.get_route_key()) {
            Some(handle) => handle.clone(),
            None => {
                let mut res = Response::new();
                res.with_status(Status::NotFound)
                    .with_content("Not found".to_owned());
                return Self::respond(&mut stream, &mut res);
            }
        };

        let mut res = handle(req);
        Self::respond(&mut stream, &mut res)?;

        Ok(())
    }
}
