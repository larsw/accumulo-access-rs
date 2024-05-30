use std::collections::HashSet;

#[derive(Debug, PartialEq, Clone)]
pub struct Authorizations {
    auths: HashSet<String>,
}

impl Authorizations {
    /// Creates a new `Authorizations` instance.
    /// 
    /// # Arguments 
    /// 
    /// * `authorizations`: A vector of authorizations.
    /// 
    /// returns: Authorizations 
    /// 
    /// # Examples 
    /// 
    /// ```
    /// use std::collections::HashSet;
    /// use accumulo_access::Authorizations;
    ///
    /// let authorizations = Authorizations::of(&["label1".to_string(), "label5".to_string()]);
    ///
    /// let expected = HashSet::from_iter(vec!["label1".to_string(), "label5".to_string()]);
    /// assert_eq!(authorizations.to_set(), expected);
    /// ```
    pub fn of(authorizations: &[String]) -> Self {
        Authorizations {
            auths: authorizations.iter().cloned().collect()
        }
    }
    
    pub fn contains(&self, auth: &str) -> bool {
        self.auths.contains(auth)
    }
    
    pub fn to_set(&self) -> HashSet<String> {
        self.auths.clone()
    }
}
