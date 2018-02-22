use futures::Stream;
use std::io;
use hyper;
use hyper::Chunk;
use hyper::server::{Http, Service};
use tokio_core::reactor::Core;
use tokio_core::net::{TcpListener, TcpStream};
use ArcCore::ReactorHandler;
use ArcRouting::{ArcRouter, RouteGroup, Router};
use std::sync::{Arc, Mutex};
use futures::Future;
use futures::prelude::{async_block, await};
use futures::task::{self, Task};
use std::net::SocketAddr;
use std::thread;
use native_tls::{Pkcs12, TlsAcceptor};
use tokio_tls::{AcceptAsync, TlsAcceptorExt};
use num_cpus;

pub type ReactorAlias = Arc<Mutex<Reactor>>;

pub struct Reactor {
	pub(crate) peers: Vec<(AcceptAsync<TcpStream>, SocketAddr)>,
	pub(crate) taskHandle: Option<Task>,
}

impl Reactor {
	pub fn new() -> ReactorAlias {
		Arc::new(Mutex::new(Reactor {
			peers: Vec::new(),
			taskHandle: None,
		}))
	}
}

pub struct ArcReactor {
	port: i16,
	timeout: i8,
	RouteService: Option<ArcRouter>,
}

impl ArcReactor {
	pub fn new() -> ArcReactor {
		ArcReactor {
			port: 8080,
			timeout: 10,
			RouteService: None,
		}
	}

	pub fn router() -> ArcRouter {
		ArcRouter::new()
	}

	pub fn routeGroup(parent: &'static str) -> RouteGroup {
		RouteGroup::new(parent)
	}

	pub fn port(mut self, port: i16) -> Self {
		self.port = port;

		self
	}

	pub fn routes(mut self, routes: Router) -> Self {
		let routes = ArcRouter {
			routes: Arc::new(routes.routes),
		};
		self.RouteService = Some(routes);

		self
	}

	pub fn initiate(self) -> io::Result<()> {
		println!("[arc-reactor]: Spawning threads!");
		let reactors = spawn(self.RouteService.expect("This thing needs routes to work!"))?;
		println!("[arc-reactor]: spawned {} threads!", reactors.len());
		println!("[arc-reactor]: Starting main event loop!");
		let mut core = Core::new()?;

		println!("[arc-reactor]: Started Main event loop!");
		let handle = core.handle();

		let addr = format!("0.0.0.0:{}", self.port).parse().unwrap();
		println!("[arc-reactor]: Binding to port {}", self.port);
		let listener = match TcpListener::bind(&addr, &handle) {
			Ok(listener) => listener,
			Err(e) => {
				eprintln!(
					"[arc-reactor]: Whoops! something else is running on port {}, {}",
					&self.port, e
				);
				return Err(e);
			}
		};

		let mut counter = 0;

		// for TLS support
		let der = include_bytes!("identity.p12");
		let cert = Pkcs12::from_der(der, "mypass").unwrap();
		let acceptor = TlsAcceptor::builder(cert).unwrap().build().unwrap();
		// let acceptor = Arc::new(acceptor);
		println!("[arc-reactor]: Running Main Event loop");
		core.run(listener.incoming().for_each(move |(socket, peerIp)| {
			let mut reactor = reactors[counter].lock().unwrap();
			let socket = acceptor.accept_async(socket);
			reactor.peers.push((socket, peerIp));

			if let Some(ref task) = reactor.taskHandle {
				task.notify();
			}

			counter += 1;
			if counter == reactors.len() {
				counter = 0
			}
			Ok(())
		}))?;

		Ok(())
	}
}

fn spawn(RouteService: ArcRouter) -> io::Result<Vec<ReactorAlias>> {
	let mut reactors = Vec::new();
	let routeService = Arc::new(RouteService);

	for _ in 0..num_cpus::get() * 2 {
		let reactor = Reactor::new();
		reactors.push(reactor.clone());
		let routeService = routeService.clone();

		thread::spawn(move || {
			let mut core = Core::new().expect("Could not start event loop");
			let handle = core.handle();
			let http: Http<Chunk> = Http::new();
			let http = Arc::new(http);

			core
				.run(ReactorHandler {
					handler: || {
						let mut reactor = reactor.lock().unwrap();
						for (socket, _peerAddr) in reactor.peers.drain(..) {
							let future = futureFactory(socket, http.clone(), routeService.clone());
							// let future = http.serve_connection(socket, routeService.clone());
							let future = future
								.map(|_stuff| ())
								.map_err(|err| println!("[arc-reactor][error]: {:?}", err));
							handle.spawn(future);
						}
						reactor.taskHandle = Some(task::current());
					},
				})
				.expect("Could not spawn threads!");
		});
	}

	Ok(reactors)
}

fn futureFactory(
	io: AcceptAsync<TcpStream>,
	http: Arc<Http<Chunk>>,
	s: Arc<ArcRouter>,
) -> impl Future<Item = (), Error = ()> {
	io.then(move |socket| {
		match socket {
			Ok(socket) => {
				println!("Serving Connection!");
				http.serve_connection(socket, s.clone())
			},
			Err(err) => {
				panic!("Failed to handshake with client aborting! {}", err)
			}
		}
	})
		.map(|_| ())
		.map_err(|err| println!("[arc-reactor][error]: {:?}", err))
}
