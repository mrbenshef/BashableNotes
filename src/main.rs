extern crate iron;
extern crate mount;
extern crate pulldown_cmark;
extern crate router;
extern crate staticfile;
extern crate time;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

extern crate shiplift;

extern crate env_logger;
#[macro_use]
extern crate log;

extern crate ws;
extern crate regex;

use iron::prelude::*;
use iron::{typemap, AfterMiddleware, BeforeMiddleware};
use iron::headers::ContentType;
use time::precise_time_ns;

use std::path::Path;
use staticfile::Static;
use mount::Mount;
use router::Router;

mod parser;

mod websocket;
use websocket::Server;
use ws::{listen};
use std::thread;

struct ResponseTime;

impl typemap::Key for ResponseTime {
    type Value = u64;
}

impl BeforeMiddleware for ResponseTime {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        req.extensions.insert::<ResponseTime>(precise_time_ns());
        Ok(())
    }
}

impl AfterMiddleware for ResponseTime {
    fn after(&self, req: &mut Request, res: Response) -> IronResult<Response> {
        let delta = precise_time_ns() - *req.extensions.get::<ResponseTime>().unwrap();
        println!("Request took: {} ms", (delta as f64) / 1000000.0);
        Ok(res)
    }
}

fn hello_world(_: &mut Request) -> IronResult<Response> {
    // Parse markdown
    // let html = parse_markdown();
    let html = include_str!("../res/index.html");

    // Serve up html
    let resp = Response::with((ContentType::html().0, iron::status::Ok, html));
    Ok(resp)
}

fn main() {
    env_logger::init();

    let ws_handle = thread::spawn(|| {
        info!("websocket thread created");
        
        let ws_address = "127.0.0.1:3012";
        info!("Starting websocket on ws://{}", ws_address);
        listen(ws_address, |out| {
            Server {
                out: out,
                ping_timeout: None,
                expire_timeout: None,
            }
        }).unwrap();
    });

    let http_handle = thread::spawn(|| {
        info!("webserver thread created");
        
        let mut chain = Chain::new(hello_world);
        chain.link_before(ResponseTime);
        chain.link_after(ResponseTime);

        let mut router = Router::new();
        router.get("/", chain, "document");

        // Serve the shared JS/CSS at /
        let mut mount = Mount::new();
        mount.mount("/", router);
        mount.mount("res/", Static::new(Path::new("res/")));
        mount.mount("notebook/", Static::new(Path::new("notebook/")));

        let address = "localhost:3000";
        info!("Starting server on http://{}", address);
        Iron::new(mount).http(address).unwrap();
    });

    ws_handle.join().unwrap();
    http_handle.join().unwrap();
}
