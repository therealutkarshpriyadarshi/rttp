//! Request context and type-safe data storage
//!
//! This module provides:
//! - Request-scoped data
//! - Type-safe extension storage
//! - Parameter extraction

use crate::http::Request;
use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Type-safe extensions map for storing arbitrary data
#[derive(Default)]
pub struct Extensions {
    map: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Extensions {
    /// Create a new empty extensions map
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Insert a value into the extensions map
    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) {
        self.map.insert(TypeId::of::<T>(), Box::new(value));
    }

    /// Get a reference to a value from the extensions map
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_ref::<T>())
    }

    /// Get a mutable reference to a value from the extensions map
    pub fn get_mut<T: Send + Sync + 'static>(&mut self) -> Option<&mut T> {
        self.map
            .get_mut(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_mut::<T>())
    }

    /// Remove a value from the extensions map
    pub fn remove<T: Send + Sync + 'static>(&mut self) -> Option<T> {
        self.map
            .remove(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast::<T>().ok())
            .map(|boxed| *boxed)
    }

    /// Check if the extensions map contains a value of type T
    pub fn contains<T: Send + Sync + 'static>(&self) -> bool {
        self.map.contains_key(&TypeId::of::<T>())
    }
}

/// Path parameters extracted from route patterns
#[derive(Debug, Clone, Default)]
pub struct Params {
    params: HashMap<String, String>,
}

impl Params {
    /// Create a new empty params collection
    pub fn new() -> Self {
        Self {
            params: HashMap::new(),
        }
    }

    /// Insert a parameter
    pub fn insert(&mut self, key: String, value: String) {
        self.params.insert(key, value);
    }

    /// Get a parameter by name
    pub fn get(&self, key: &str) -> Option<&str> {
        self.params.get(key).map(|s| s.as_str())
    }

    /// Get all parameters
    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.params.iter()
    }

    /// Check if parameters are empty
    pub fn is_empty(&self) -> bool {
        self.params.is_empty()
    }

    /// Get the number of parameters
    pub fn len(&self) -> usize {
        self.params.len()
    }
}

impl FromIterator<(String, String)> for Params {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(iter: T) -> Self {
        Self {
            params: iter.into_iter().collect(),
        }
    }
}

/// Request context containing request data and extensions
pub struct Context {
    request: Request,
    params: Params,
    extensions: Extensions,
}

impl Context {
    /// Create a new context from a request
    pub fn new(request: Request) -> Self {
        Self {
            request,
            params: Params::new(),
            extensions: Extensions::new(),
        }
    }

    /// Create a new context with parameters
    pub fn with_params(request: Request, params: Params) -> Self {
        Self {
            request,
            params,
            extensions: Extensions::new(),
        }
    }

    /// Get a reference to the request
    pub fn request(&self) -> &Request {
        &self.request
    }

    /// Get a mutable reference to the request
    pub fn request_mut(&mut self) -> &mut Request {
        &mut self.request
    }

    /// Get a reference to the path parameters
    pub fn params(&self) -> &Params {
        &self.params
    }

    /// Get a mutable reference to the path parameters
    pub fn params_mut(&mut self) -> &mut Params {
        &mut self.params
    }

    /// Get a path parameter by name
    pub fn param(&self, name: &str) -> Option<&str> {
        self.params.get(name)
    }

    /// Get a reference to the extensions
    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    /// Get a mutable reference to the extensions
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }

    /// Parse the request body as JSON
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_slice(self.request.body())
    }

    /// Get query parameter from URI
    pub fn query(&self, name: &str) -> Option<&str> {
        let uri = self.request.uri();
        let query_start = uri.find('?')?;
        let query_string = &uri[query_start + 1..];

        for pair in query_string.split('&') {
            let mut parts = pair.split('=');
            if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                if key == name {
                    return Some(value);
                }
            }
        }
        None
    }

    /// Consume the context and return the inner request
    pub fn into_request(self) -> Request {
        self.request
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::Method;

    #[test]
    fn test_extensions_insert_and_get() {
        let mut ext = Extensions::new();
        ext.insert(42i32);
        ext.insert("hello".to_string());

        assert_eq!(ext.get::<i32>(), Some(&42));
        assert_eq!(ext.get::<String>(), Some(&"hello".to_string()));
        assert_eq!(ext.get::<f64>(), None);
    }

    #[test]
    fn test_extensions_get_mut() {
        let mut ext = Extensions::new();
        ext.insert(42i32);

        if let Some(val) = ext.get_mut::<i32>() {
            *val = 100;
        }

        assert_eq!(ext.get::<i32>(), Some(&100));
    }

    #[test]
    fn test_extensions_remove() {
        let mut ext = Extensions::new();
        ext.insert(42i32);

        assert_eq!(ext.remove::<i32>(), Some(42));
        assert_eq!(ext.get::<i32>(), None);
    }

    #[test]
    fn test_extensions_contains() {
        let mut ext = Extensions::new();
        ext.insert(42i32);

        assert!(ext.contains::<i32>());
        assert!(!ext.contains::<String>());
    }

    #[test]
    fn test_params_basic() {
        let mut params = Params::new();
        params.insert("id".to_string(), "123".to_string());
        params.insert("name".to_string(), "john".to_string());

        assert_eq!(params.get("id"), Some("123"));
        assert_eq!(params.get("name"), Some("john"));
        assert_eq!(params.get("age"), None);
        assert_eq!(params.len(), 2);
        assert!(!params.is_empty());
    }

    #[test]
    fn test_params_from_iter() {
        let params: Params = vec![
            ("id".to_string(), "123".to_string()),
            ("name".to_string(), "john".to_string()),
        ]
        .into_iter()
        .collect();

        assert_eq!(params.get("id"), Some("123"));
        assert_eq!(params.get("name"), Some("john"));
    }

    #[test]
    fn test_context_creation() {
        let request = Request::new(Method::GET, "/test".to_string());
        let ctx = Context::new(request);

        assert_eq!(ctx.request().uri(), "/test");
        assert!(ctx.params().is_empty());
    }

    #[test]
    fn test_context_with_params() {
        let request = Request::new(Method::GET, "/users/123".to_string());
        let mut params = Params::new();
        params.insert("id".to_string(), "123".to_string());

        let ctx = Context::with_params(request, params);

        assert_eq!(ctx.param("id"), Some("123"));
    }

    #[test]
    fn test_context_extensions() {
        let request = Request::new(Method::GET, "/test".to_string());
        let mut ctx = Context::new(request);

        ctx.extensions_mut().insert(42i32);
        ctx.extensions_mut().insert("user_id".to_string());

        assert_eq!(ctx.extensions().get::<i32>(), Some(&42));
        assert_eq!(
            ctx.extensions().get::<String>(),
            Some(&"user_id".to_string())
        );
    }

    #[test]
    fn test_context_query() {
        let request = Request::new(Method::GET, "/search?q=rust&page=2".to_string());
        let ctx = Context::new(request);

        assert_eq!(ctx.query("q"), Some("rust"));
        assert_eq!(ctx.query("page"), Some("2"));
        assert_eq!(ctx.query("limit"), None);
    }
}
