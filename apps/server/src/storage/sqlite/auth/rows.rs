use super::models::{AuthSessionRecord, AuthUserRecord, LocalAuthIdentityRecord};

pub(crate) fn map_auth_user_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AuthUserRecord> {
    Ok(AuthUserRecord {
        id: row.get(0)?,
        username: row.get(1)?,
        email: row.get(2)?,
        display_name: row.get(3)?,
        role: row.get(4)?,
        status: row.get(5)?,
        must_change_password: row.get::<_, i64>(6)? != 0,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

pub(crate) fn map_local_auth_identity_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<LocalAuthIdentityRecord> {
    Ok(LocalAuthIdentityRecord {
        user: map_auth_user_row(row)?,
        password_hash: row.get(9)?,
    })
}

pub(crate) fn map_auth_session_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AuthSessionRecord> {
    Ok(AuthSessionRecord {
        user: map_auth_user_row(row)?,
        expires_at: row.get(9)?,
    })
}
