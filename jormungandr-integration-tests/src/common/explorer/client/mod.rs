use super::GraphQLQuery;

pub struct GraphQLClient {
    base_url: String,
}

error_chain! {
    foreign_links {
        Reqwest(reqwest::Error);
    }
}

impl GraphQLClient {
    pub fn new<S: Into<String>>(base_address: S) -> GraphQLClient {
        let base_url = format!("http://{}/explorer/graphql", base_address.into());
        GraphQLClient { base_url }
    }

    pub fn run(&self, query: GraphQLQuery) -> Result<reqwest::Response> {
        println!("running query: {:?}, against: {}", query, self.base_url);
        reqwest::Client::new()
            .post(&format!("{}", self.base_url))
            .json(&query)
            .send()
            .map_err(|err| err.into())
    }
}
