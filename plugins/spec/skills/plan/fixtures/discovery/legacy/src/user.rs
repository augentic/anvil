// src/user.rs — user lifecycle for the legacy monolith.
//
// Surfaces two capabilities:
//   - registration       (sign-up flow; inserts a new user row)
//   - email_verification (one-time link confirmation; flips `active`)
//
// email_verification depends on registration: the verification token
// is issued during registration and only matters once a user row
// exists.

pub struct User {
    pub id: u64,
    pub email: String,
    pub password_hash: String,
    pub active: bool,
}

pub struct VerificationToken {
    pub user_id: u64,
    pub token: String,
}

/// User registration.
///
/// Capability: `registration`.
/// Creates a new `User` row with `active = false` and issues a
/// `VerificationToken` for the email-verification flow.
pub fn register(email: &str, password: &str) -> User {
    let user = User {
        id: next_user_id(),
        email: email.to_string(),
        password_hash: hash_password(password),
        active: false,
    };
    issue_verification_token(&user);
    user
}

/// Email verification.
///
/// Capability: `email_verification` (depends on `registration`).
/// Consumes a `VerificationToken` and flips the owning user's
/// `active` flag.
pub fn verify_email(token: &str) -> Result<(), &'static str> {
    let vt = lookup_token(token).ok_or("unknown token")?;
    let mut user = lookup_user(vt.user_id).ok_or("unknown user")?;
    user.active = true;
    save_user(&user);
    Ok(())
}

fn next_user_id() -> u64 { unimplemented!() }
fn hash_password(_p: &str) -> String { unimplemented!() }
fn issue_verification_token(_u: &User) {}
fn lookup_token(_t: &str) -> Option<VerificationToken> { None }
fn lookup_user(_id: u64) -> Option<User> { None }
fn save_user(_u: &User) {}
