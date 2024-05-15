use std::cmp::Ordering;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub enum AuthorizationExpression {
    And(Vec<AuthorizationExpression>),
    Or(Vec<AuthorizationExpression>),
    AccessToken(String),
}

impl Ord for AuthorizationExpression {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (AuthorizationExpression::And(_), AuthorizationExpression::Or(_)) => Ordering::Less,
            (AuthorizationExpression::Or(_), AuthorizationExpression::And(_)) => Ordering::Greater,
            (AuthorizationExpression::And(a), AuthorizationExpression::And(b)) => a.cmp(b),
            (AuthorizationExpression::Or(a), AuthorizationExpression::Or(b)) => a.cmp(b),
            (AuthorizationExpression::AccessToken(a), AuthorizationExpression::AccessToken(b)) => a.cmp(b),
            (AuthorizationExpression::AccessToken(_), _) => Ordering::Greater,
            (_, AuthorizationExpression::AccessToken(_)) => Ordering::Less,
        }
    }
}

impl PartialOrd for AuthorizationExpression {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for AuthorizationExpression {}

impl PartialEq for AuthorizationExpression {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}


impl AuthorizationExpression {
    pub fn new(json: &serde_json::Value) -> Result<Self, String> {
        match json {
            serde_json::Value::Object(obj) => {
                if obj.contains_key("and") {
                    let and = obj.get("and").unwrap().as_array().unwrap();
                    let mut nodes = Vec::new();
                    for node in and {
                        nodes.push(AuthorizationExpression::new(node)?);
                    }
                    Ok(AuthorizationExpression::And(nodes))
                } else if obj.contains_key("or") {
                    let or = obj.get("or").unwrap().as_array().unwrap();
                    let mut nodes = Vec::new();
                    for node in or {
                        nodes.push(AuthorizationExpression::new(node)?);
                    }
                    Ok(AuthorizationExpression::Or(nodes))
                } else {
                    Err("Invalid JSON object".to_string())
                }
            }
            serde_json::Value::String(token) => Ok(AuthorizationExpression::AccessToken(token.to_string())),
            _ => Err("Invalid JSON value".to_string()),
        }
    }
    
    pub fn evaluate(&self, authorizations: &HashSet<String>) -> bool {
        match self {
            AuthorizationExpression::And(nodes) =>
                nodes.iter().all(|node| node.evaluate(authorizations)),

            AuthorizationExpression::Or(nodes) =>
                nodes.iter().any(|node| node.evaluate(authorizations)),

            AuthorizationExpression::AccessToken(token) => authorizations.contains(token),
        }
    }

    pub fn to_json_str(&self) -> String {
        match self {
            AuthorizationExpression::And(nodes) => {
                let mut json = String::from("{\"and\": [");
                for node in nodes {
                    json.push_str(&node.to_json_str());
                    json.push(',');
                }
                json.pop();
                json.push(']');
                json.push('}');
                json
            }
            AuthorizationExpression::Or(nodes) => {
                let mut json = String::from("{\"or\": [");
                for node in nodes {
                    json.push_str(&node.to_json_str());
                    json.push(',');
                }
                json.pop();
                json.push(']');
                json.push('}');
                json
            }
            AuthorizationExpression::AccessToken(token) => format!("\"{}\"", token),
        }
    }
    
    // serialize the expression to json with serde
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            AuthorizationExpression::And(nodes) => {
                let mut json = serde_json::json!({"and": []});
                let and = json.as_object_mut().unwrap().get_mut("and").unwrap();
                for node in nodes {
                    and.as_array_mut().unwrap().push(node.to_json());
                }
                json
            }
            AuthorizationExpression::Or(nodes) => {
                let mut json = serde_json::json!({"or": []});
                let or = json.as_object_mut().unwrap().get_mut("or").unwrap();
                for node in nodes {
                    or.as_array_mut().unwrap().push(node.to_json());
                }
                json
            }
            AuthorizationExpression::AccessToken(token) => serde_json::json!(token),
        }
    }

    pub fn to_expression_str(&self) -> String {
        // serialize the expression tree back as a valid Accumulo Security Expression including parentheses, optional quotes, '&' and '|'.
        match self {
            AuthorizationExpression::And(nodes) => {
                let mut expression = String::new();
                for node in nodes {
                    expression.push_str(&node.to_expression_str());
                    expression.push_str(" & ");
                }
                expression.pop();
                expression.pop();
                expression
            }
            AuthorizationExpression::Or(nodes) => {
                let mut expression = String::new();
                for node in nodes {
                    expression.push_str(&node.to_expression_str());
                    expression.push_str(" | ");
                }
                expression.pop();
                expression.pop();
                expression
            }
            AuthorizationExpression::AccessToken(token) => token.clone(),
        }
    }

    /// sort and normalize (remove duplicates) in the expression tree.
    pub fn normalize(&mut self) {
        match self {
            AuthorizationExpression::And(nodes) => {
                nodes.sort();
                nodes.dedup();
                for node in nodes {
                    node.normalize();
                }
            }
            AuthorizationExpression::Or(nodes) => {
                nodes.sort();
                nodes.dedup();
                for node in nodes {
                    node.normalize();
                }
            }
            AuthorizationExpression::AccessToken(_) => {}
        }
    }
}