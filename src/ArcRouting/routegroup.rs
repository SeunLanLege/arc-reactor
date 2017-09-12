 use ArcProto::*;

use hyper::{Method};
use std::collections::HashMap;

pub struct RouteGroup {
	pub(crate) parent: &'static str,
	pub(crate) routes: HashMap<String, (Method, Box<ArcService>)>,
}


impl RouteGroup {
	pub fn new(parent: &'static str) -> Self {
		RouteGroup {
			parent,
			routes: HashMap::new(),
		}
	}

	pub fn add(mut self, group: RouteGroup) -> Self {
		let RouteGroup { routes, .. } = group;

		for (path, (method, handler)) in routes.into_iter() {
			self.routes
			  .insert(path, (method, handler));
		}

		self
	}
	
	pub fn get<S: ArcService + 'static + Send + Sync>(self, route: &'static str, handler: S) -> Self {
		self.route(Method::Get, route, handler)
	}
	
	pub fn post<S: ArcService + 'static + Send + Sync>(self, route: &'static str, handler: S) -> Self {
		self.route(Method::Post, route, handler)
	}
	
	pub fn put<S: ArcService + 'static + Send + Sync>(self, route: &'static str, handler: S) -> Self {
		self.route(Method::Put, route, handler)
	}
	
	pub fn patch<S: ArcService + 'static + Send + Sync>(self, route: &'static str, handler: S) -> Self {
		self.route(Method::Patch, route, handler)
	}
	
	pub fn delete<S: ArcService + 'static + Send + Sync>(self, route: &'static str, handler: S) -> Self {
		self.route(Method::Delete, route, handler)
	}
	
	fn route<S: ArcService + 'static + Send + Sync>(mut self, method: Method, path: &'static str, handler: S) -> Self {
		self.routes
		.insert(format!("/{}{}", &self.parent, path), (method, Box::new(handler)));
	
		self
	}
}