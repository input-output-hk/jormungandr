use actix_web::pred::Predicate;
use actix_web::Request;
use regex::{self, Regex};

pub struct PathPredicate {
    regex: Regex,
}

impl PathPredicate {
    pub fn new(prefix: &str, pattern: &str) -> Self {
        let full_pattern = format!("^/{}/{}$", regex::escape(prefix), pattern);
        let regex = Regex::new(&full_pattern).unwrap_or_else(|e| {
            panic!(
                "Error while creating regex for prefix '{}' and pattern'{}': {}",
                prefix, pattern, e
            )
        });
        Self { regex }
    }
}

impl<T> Predicate<T> for PathPredicate {
    fn check(&self, req: &Request, _: &T) -> bool {
        self.regex.is_match(req.path())
    }
}
