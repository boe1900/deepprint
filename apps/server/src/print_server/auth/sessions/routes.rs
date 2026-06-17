#[path = "routes/change_password.rs"]
mod change_password;
#[path = "routes/login.rs"]
mod login;
#[path = "routes/logout.rs"]
mod logout;
#[path = "routes/me.rs"]
mod me;

pub(crate) use change_password::auth_change_password;
pub(crate) use login::auth_login;
pub(crate) use logout::auth_logout;
pub(crate) use me::auth_me;
