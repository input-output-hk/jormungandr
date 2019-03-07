use actix_web::pred::Predicate;
use actix_web::Request;
use regex::{self, Regex};

pub struct PathPredicate {
    regex: Regex,
}

impl PathPredicate {
    // The pattern should be a full URI, with or without slash on the beginning.
    // Wildcards in form of `{some_name}` are allowed, they accept everything except `/`.
    // The predicate will accept only requests with URIs fully matching the pattern.
    pub fn for_pattern(pattern: &str) -> Self {
        let pattern = regex::escape(&pattern);
        let segment_regex = Regex::new(r"\\\{[^}]*\}").unwrap();
        let pattern = segment_regex.replace_all(&pattern, r"[^/]*");
        let prefix = match pattern.starts_with('/') {
            true => "",
            false => "/",
        };
        let pattern = format!("^{}{}$", prefix, pattern);
        Self {
            regex: Regex::new(&pattern).unwrap(),
        }
    }
}

impl<T> Predicate<T> for PathPredicate {
    fn check(&self, req: &Request, _: &T) -> bool {
        self.regex.is_match(req.path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;

    fn assert_uri_accepted(pattern: &str, uri: &str) {
        let is_accepted = is_uri_accepted(pattern, uri);

        assert!(
            is_accepted,
            "Predicate for pattern '{}' rejected URI '{}'",
            pattern, uri
        );
    }

    fn assert_uri_rejected(pattern: &str, uri: &str) {
        let is_accepted = is_uri_accepted(pattern, uri);

        assert!(
            !is_accepted,
            "Predicate for pattern '{}' accepted URI '{}'",
            pattern, uri
        );
    }

    fn is_uri_accepted(pattern: &str, uri: &str) -> bool {
        let predicate = PathPredicate::for_pattern(pattern);
        let request = TestRequest::with_uri(uri).finish();
        predicate.check(&request, &())
    }

    #[test]
    fn path_predicate_accepts_only_matching_uris() {
        assert_uri_rejected("a", "/");
        assert_uri_accepted("a", "/a");
        assert_uri_rejected("a", "/b");
        assert_uri_rejected("a", "/a/b");
        assert_uri_rejected("a/{X}", "/a");
        assert_uri_accepted("a/{X}", "/a/b");
        assert_uri_rejected("a/{X}", "/a/b/c");
        assert_uri_rejected("a/{X}/c", "/a/b");
        assert_uri_accepted("a/{X}/c", "/a/b/c");
        assert_uri_rejected("a/{X}/c", "/a/b/c/c");
        assert_uri_accepted("/a", "/a");
    }
}
