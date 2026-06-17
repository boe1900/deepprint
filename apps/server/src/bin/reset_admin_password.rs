use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use rand_core::OsRng;
use rusqlite::{params, Connection};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = std::env::args()
        .nth(1)
        .ok_or("usage: reset_admin_password <db_path> <new_password>")?;
    let new_password = std::env::args()
        .nth(2)
        .ok_or("usage: reset_admin_password <db_path> <new_password>")?;

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(new_password.as_bytes(), &salt)
        .map_err(|err| err.to_string())?
        .to_string();

    let conn = Connection::open(db_path)?;
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE auth_identities
         SET password_hash = ?1, updated_at = strftime('%s','now')
         WHERE user_id = (
           SELECT id FROM users WHERE username = 'admin'
         )
         AND provider_type = 'local'
         AND provider_key = 'admin'",
        params![password_hash],
    )?;
    tx.execute(
        "UPDATE users
         SET must_change_password = 0,
             password_changed_at = strftime('%s','now'),
             updated_at = strftime('%s','now')
         WHERE username = 'admin'",
        [],
    )?;
    tx.commit()?;

    println!("admin password reset");
    Ok(())
}
