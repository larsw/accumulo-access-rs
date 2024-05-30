use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub enum AuthorizationExpression {
    /// A conjunction of multiple access tokens or scopes.
    ConjunctionOf(Vec<AuthorizationExpression>),
    /// A disjunction of multiple access tokens or scopes.
    DisjunctionOf(Vec<AuthorizationExpression>),
    /// An access token.
    AccessToken(String),
    /// A nil expression (empty string).
    Nil
}

impl Hash for AuthorizationExpression {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            AuthorizationExpression::ConjunctionOf(nodes) => {
                let mut sorted = nodes.clone();
                sorted.sort();
                state.write_u8(0);
                sorted.hash(state);
            }
            AuthorizationExpression::DisjunctionOf(nodes) => {
                let mut sorted = nodes.clone();
                sorted.sort();
                state.write_u8(1);
                sorted.hash(state);
            }
            AuthorizationExpression::AccessToken(token) => {
                state.write_u8(2);
                token.hash(state);
            }
            AuthorizationExpression::Nil => {
                state.write_u8(3);
            }
        }
    }
}

impl Ord for AuthorizationExpression {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (AuthorizationExpression::ConjunctionOf(a), AuthorizationExpression::DisjunctionOf(b)) => a.cmp(b),
            (AuthorizationExpression::DisjunctionOf(a), AuthorizationExpression::ConjunctionOf(b)) => a.cmp(b),
            (AuthorizationExpression::ConjunctionOf(a), AuthorizationExpression::ConjunctionOf(b)) => a.cmp(b),
            (AuthorizationExpression::DisjunctionOf(a), AuthorizationExpression::DisjunctionOf(b)) => a.cmp(b),
            (AuthorizationExpression::AccessToken(a), AuthorizationExpression::AccessToken(b)) => a.cmp(b),
            (AuthorizationExpression::AccessToken(_), _) => Ordering::Greater,
            (AuthorizationExpression::ConjunctionOf(_), AuthorizationExpression::AccessToken(_)) => Ordering::Less,
            (AuthorizationExpression::DisjunctionOf(_), AuthorizationExpression::AccessToken(_)) => Ordering::Less,
            (_, AuthorizationExpression::AccessToken(_)) => Ordering::Less,
            (_, AuthorizationExpression::Nil) => Ordering::Equal,
            (AuthorizationExpression::Nil, _) => Ordering::Equal
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
        match (self, other) {
            (AuthorizationExpression::ConjunctionOf(a), AuthorizationExpression::ConjunctionOf(b)) => {
                let self_set: HashSet<_> = a.iter().collect();
                let other_set: HashSet<_> = b.iter().collect();
                self_set == other_set
            },
            (AuthorizationExpression::DisjunctionOf(a), AuthorizationExpression::DisjunctionOf(b)) => {
                let self_set: HashSet<_> = a.iter().collect();
                let other_set: HashSet<_> = b.iter().collect();
                self_set == other_set
            },
            (AuthorizationExpression::AccessToken(a), AuthorizationExpression::AccessToken(b)) => a == b,
            (AuthorizationExpression::Nil, AuthorizationExpression::Nil) => true,
            _ => false,
        }
    }
}

impl Display for AuthorizationExpression {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_expression_str().as_str())
    }
}

impl AuthorizationExpression {
    /// Create a new `AuthorizationExpression` from a JSON value.
    /// 
    /// # Arguments
    /// json - The JSON value to parse.
    /// 
    /// # Returns
    /// A new `AuthorizationExpression` instance.
    /// 
    /// # Example
    /// ```
    /// use accumulo_access::AuthorizationExpression;
    /// let json = serde_json::json!({
    ///   "and": [
    ///    "A",
    ///   {
    ///     "or": [
    ///       "B",
    ///      "C"
    ///    ]
    ///  }
    /// ]
    /// });
    /// let expr = AuthorizationExpression::from_json(&json).unwrap();
    /// ```
    pub fn from_json(json: &serde_json::Value) -> Result<Self, String> {
        match json {
            serde_json::Value::Object(obj) => {
                if obj.contains_key("and") {
                    let and = obj.get("and").unwrap().as_array().unwrap();
                    let mut nodes = Vec::new();
                    for node in and {
                        nodes.push(AuthorizationExpression::from_json(node)?);
                    }
                    Ok(AuthorizationExpression::ConjunctionOf(nodes))
                } else if obj.contains_key("or") {
                    let or = obj.get("or").unwrap().as_array().unwrap();
                    let mut nodes = Vec::new();
                    for node in or {
                        nodes.push(AuthorizationExpression::from_json(node)?);
                    }
                    Ok(AuthorizationExpression::DisjunctionOf(nodes))
                } else {
                    Err("Invalid JSON object".to_string())
                }
            }
            serde_json::Value::String(token) => Ok(AuthorizationExpression::AccessToken(token.to_string())),
            _ => Err("Invalid JSON value".to_string()),
        }
    }

    /// Evaluate the expression with the given set of authorizations.
    /// Returns `true` if the authorizations are valid, `false` otherwise.
    /// 
    /// # Arguments
    /// authorizations - The set of authorizations to check.
    /// 
    /// # Example
    /// ```
    /// use std::collections::HashSet;
    /// use accumulo_access::AuthorizationExpression;
    /// let expr = AuthorizationExpression::ConjunctionOf(vec![
    ///    AuthorizationExpression::AccessToken("A".to_string()),
    ///   AuthorizationExpression::DisjunctionOf(vec![
    ///      AuthorizationExpression::AccessToken("B".to_string()),
    ///     AuthorizationExpression::AccessToken("C".to_string()),
    /// ]),
    /// ]);
    /// let authorizations = HashSet::from([
    ///    "A".to_string(),
    ///   "B".to_string(),
    /// ]);
    /// assert_eq!(expr.evaluate(&authorizations), true);
    /// ```
    pub fn evaluate(&self, authorizations: &HashSet<String>) -> bool {
        match self {
            AuthorizationExpression::Nil => true,

            AuthorizationExpression::ConjunctionOf(nodes) =>
                nodes.iter().all(|node| node.evaluate(authorizations)),

            AuthorizationExpression::DisjunctionOf(nodes) =>
                nodes.iter().any(|node| node.evaluate(authorizations)),

            AuthorizationExpression::AccessToken(token) => authorizations.contains(token),
        }
    }


    /// Create a JSON representation of the expression tree.
    /// 
    /// # Returns
    /// A JSON value representing the expression tree.
    /// 
    /// # Example
    /// ```
    /// use accumulo_access::AuthorizationExpression;
    /// let expr = AuthorizationExpression::ConjunctionOf(vec![
    ///   AuthorizationExpression::AccessToken("A".to_string()),
    ///  AuthorizationExpression::AccessToken("B".to_string()),
    /// ]);
    /// let json = expr.to_json();
    /// assert_eq!(json, serde_json::json!({"and": ["A", "B"]}));
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            AuthorizationExpression::Nil => serde_json::Value::Null,
            AuthorizationExpression::ConjunctionOf(nodes) => {
                let mut json = serde_json::json!({"and": []});
                let and = json.as_object_mut().unwrap().get_mut("and").unwrap();
                for node in nodes {
                    and.as_array_mut().unwrap().push(node.to_json());
                }
                json
            }
            AuthorizationExpression::DisjunctionOf(nodes) => {
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

    /// Create a JSON string representation of the expression tree.
    /// 
    /// # Returns
    /// A JSON string representing the expression tree.
    /// 
    /// # Example
    /// ```
    /// use accumulo_access::AuthorizationExpression;
    /// let expr = AuthorizationExpression::ConjunctionOf(vec![
    ///  AuthorizationExpression::AccessToken("A".to_string()),
    ///  AuthorizationExpression::AccessToken("B".to_string()),
    /// ]);
    /// 
    /// let json_str = expr.to_json_str();
    /// assert_eq!(json_str, "{\"and\":[\"A\",\"B\"]}");
    pub fn to_json_str(&self) -> String {
        self.to_json().to_string()
    }

    /// Create a string representation of the expression tree.
    /// 
    /// # Returns
    /// A string representing the expression tree.
    /// 
    /// # Example
    /// ```
    /// use accumulo_access::AuthorizationExpression;
    /// let expr1 = AuthorizationExpression::ConjunctionOf(vec![
    /// AuthorizationExpression::AccessToken("A".to_string()),
    /// AuthorizationExpression::AccessToken("B".to_string()),
    /// ]);
    /// 
    /// let expr_str = expr1.to_expression_str();
    /// assert_eq!(expr_str, "A&B");    ///
    ///
    /// let expr2 = AuthorizationExpression::DisjunctionOf(vec![
    /// AuthorizationExpression::AccessToken("A".to_string()),
    /// AuthorizationExpression::AccessToken("B".to_string()),
    /// ]);
    ///
    /// let expr_str = expr2.to_expression_str();
    /// assert_eq!(expr_str, "A|B");
    pub fn to_expression_str(&self) -> String {
        // serialize the expression tree back as a valid Accumulo Security Expression including parentheses, optional quotes, '&' and '|'.
        match self {
            AuthorizationExpression::Nil => String::new(),
            AuthorizationExpression::ConjunctionOf(nodes) => {
                let mut expression = String::new();
                for node in nodes {
                    expression.push_str(&node.to_expression_str());
                    expression.push('&');
                }
                expression.pop();
                expression
            }
            AuthorizationExpression::DisjunctionOf(nodes) => {
                let mut expression = String::new();
                for node in nodes {
                    expression.push_str(&node.to_expression_str());
                    expression.push('|');
                }
                expression.pop();
                expression
            }
            AuthorizationExpression::AccessToken(token) => token.clone(),
        }
    }

    /// Normalize the expression tree by sorting and deduplicating the nodes.
    /// 
    /// # Example
    /// ```
    /// use accumulo_access::AuthorizationExpression;
    /// let mut expr = AuthorizationExpression::ConjunctionOf(vec![
    /// AuthorizationExpression::AccessToken("B".to_string()),
    /// AuthorizationExpression::AccessToken("A".to_string()),
    /// AuthorizationExpression::AccessToken("B".to_string()),
    /// AuthorizationExpression::DisjunctionOf(vec![
    /// AuthorizationExpression::AccessToken("C".to_string()),
    /// AuthorizationExpression::AccessToken("D".to_string()),
    /// AuthorizationExpression::AccessToken("D".to_string())]
    /// )]);
    /// expr.normalize();
    /// let expected = AuthorizationExpression::ConjunctionOf(vec![
    /// AuthorizationExpression::AccessToken("A".to_string()),
    /// AuthorizationExpression::AccessToken("B".to_string()),
    /// AuthorizationExpression::DisjunctionOf(vec![
    /// AuthorizationExpression::AccessToken("C".to_string()),
    /// AuthorizationExpression::AccessToken("D".to_string())]
    /// )]);
    /// 
    /// assert_eq!(expr, expected);
    pub fn normalize(&mut self) {
        match self {
            AuthorizationExpression::Nil => {},

            AuthorizationExpression::ConjunctionOf(nodes) => {
                nodes.sort();
                nodes.dedup();
                for node in nodes {
                    node.normalize();
                }
            }
            AuthorizationExpression::DisjunctionOf(nodes) => {
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

// test for normalize
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn some_basic_equality_and_ordering_tests() {
        assert_eq!(AuthorizationExpression::AccessToken("A".to_string()), AuthorizationExpression::AccessToken("A".to_string()));
        assert_ne!(AuthorizationExpression::AccessToken("A".to_string()), AuthorizationExpression::AccessToken("B".to_string()));

        assert_eq!(AuthorizationExpression::ConjunctionOf(vec![
            AuthorizationExpression::AccessToken("A".to_string()),
            AuthorizationExpression::AccessToken("B".to_string()),
        ]), AuthorizationExpression::ConjunctionOf(vec![
            AuthorizationExpression::AccessToken("B".to_string()),
            AuthorizationExpression::AccessToken("A".to_string()),
        ]));

        assert_eq!(AuthorizationExpression::DisjunctionOf(vec![
            AuthorizationExpression::AccessToken("A".to_string()),
            AuthorizationExpression::AccessToken("B".to_string()),
        ]), AuthorizationExpression::DisjunctionOf(vec![
            AuthorizationExpression::AccessToken("B".to_string()),
            AuthorizationExpression::AccessToken("A".to_string()),
        ]));
    }

    #[test]
    fn new_expr_from_json() {
        let json = serde_json::json!({
            "and": [
                "A",
                {
                    "or": [
                        "B",
                        "C"
                    ]
                }
            ]
        });
        let expr = AuthorizationExpression::from_json(&json).unwrap();
        assert_eq!(expr, AuthorizationExpression::ConjunctionOf(vec![
            AuthorizationExpression::AccessToken("A".to_string()),
            AuthorizationExpression::DisjunctionOf(vec![
                AuthorizationExpression::AccessToken("B".to_string()),
                AuthorizationExpression::AccessToken("C".to_string()),
            ]),
        ]));
    }

    #[test]
    fn test_normalize1() {
        let mut expr = AuthorizationExpression::ConjunctionOf(vec![
            AuthorizationExpression::AccessToken("B".to_string()),
            AuthorizationExpression::AccessToken("A".to_string()),
            AuthorizationExpression::AccessToken("B".to_string()),
            AuthorizationExpression::AccessToken("B".to_string()),
            AuthorizationExpression::DisjunctionOf(vec![
                AuthorizationExpression::AccessToken("C".to_string()),
                AuthorizationExpression::AccessToken("D".to_string()),
                AuthorizationExpression::AccessToken("D".to_string()),
                AuthorizationExpression::AccessToken("D".to_string()),
                AuthorizationExpression::AccessToken("D".to_string()),
                AuthorizationExpression::AccessToken("D".to_string()),
            ]),
        ]);

        expr.normalize();

        assert_eq!(expr, AuthorizationExpression::ConjunctionOf(vec![
            AuthorizationExpression::AccessToken("B".to_string()),
            AuthorizationExpression::AccessToken("A".to_string()),
            AuthorizationExpression::DisjunctionOf(vec![
                AuthorizationExpression::AccessToken("D".to_string()),
                AuthorizationExpression::AccessToken("C".to_string()),
            ]),
        ]));
    }
}