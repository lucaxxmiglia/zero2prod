use validator::validate_email;

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse (s: String) -> Result<SubscriberEmail,String> {
        if validate_email(&s) {
            Ok(Self(s))
        } else {
            Err(format!("{} mail non valida",s))
        }
        
    }
}

impl std::fmt::Display for SubscriberEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::SubscriberEmail;
    use claim::{assert_err, assert_ok};
    use fake::faker::internet::en:: SafeEmail;
    use fake:: Fake;

    #[derive(Debug,Clone)]
    struct ValidEmailFixture(pub String);

    impl quickcheck::Arbitrary for ValidEmailFixture {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            let email = SafeEmail().fake_with_rng(g);
            Self(email)
        }
    }

    #[quickcheck_macros::quickcheck]
    fn valid_email_are_parsed_ok(valid_email:ValidEmailFixture)->bool {
        SubscriberEmail::parse(valid_email.0).is_ok()
    }

    #[test]
    fn valid_email_is_ok() {
        let email = SafeEmail().fake();
        assert_ok!(SubscriberEmail::parse(email));
    }
    
    #[test]
    fn emptystring_is_rejected() {
        let email ="".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_at_is_rejected() {
        let email ="ginopilotino.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email ="@ginopilotino.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }
}