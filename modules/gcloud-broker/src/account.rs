//! The service account tokens are minted for, and the project it lives in,
//! which is where a sandboxed command's gcloud works.

/// What Google ends a project's own service accounts' addresses with.
const DOMAIN: &str = ".iam.gserviceaccount.com";

/// A service account a project made, `name@project.iam.gserviceaccount.com`.
///
/// Google's own accounts, such as a project's default compute account,
/// live elsewhere and say no project, so they're refused: a sandbox's
/// account is one made for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Account {
    email: String,
    project: String,
}

impl Account {
    /// Reads a service account's address.
    ///
    /// # Errors
    ///
    /// When `email` isn't the address of an account a project made.
    pub fn parse(email: &str) -> Result<Account, String> {
        let refuse = || format!("{email:?} isn't a service account, name@project{DOMAIN}");
        let (name, rest) = email.split_once('@').ok_or_else(refuse)?;
        let project = rest.strip_suffix(DOMAIN).ok_or_else(refuse)?;
        // Both are lowercase letters, digits and hyphens wherever Google
        // makes them, so anything else is a mistake, or more than a name.
        let word = |text: &str| {
            !text.is_empty()
                && text
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        };
        if !word(name) || !word(project) {
            return Err(refuse());
        }
        Ok(Account {
            email: email.to_owned(),
            project: project.to_owned(),
        })
    }

    /// The account's address.
    #[must_use]
    pub fn email(&self) -> &str {
        &self.email
    }

    /// The project the account lives in.
    #[must_use]
    pub fn project(&self) -> &str {
        &self.project
    }
}

#[cfg(test)]
mod tests {
    use super::Account;

    #[test]
    fn an_account_says_its_project() {
        let account = Account::parse("agents@a-lab-4.iam.gserviceaccount.com").unwrap();
        assert_eq!(
            account.project(),
            "a-lab-4",
            "the project is the address's middle"
        );
        assert_eq!(account.email(), "agents@a-lab-4.iam.gserviceaccount.com");
    }

    #[test]
    fn what_isnt_a_projects_account_is_refused() {
        for email in [
            "someone@example.com",
            "123-compute@developer.gserviceaccount.com",
            "@lab.iam.gserviceaccount.com",
            "agents@.iam.gserviceaccount.com",
            "agents@lab.iam.gserviceaccount.com --verbosity=debug",
            "Agents@lab.iam.gserviceaccount.com",
            "agents",
        ] {
            assert!(
                Account::parse(email).is_err(),
                "{email:?} was taken as an account"
            );
        }
    }
}
