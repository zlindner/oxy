use crate::{client::Client, login::packets, packet::Packet};

use chrono::NaiveDateTime;
use deadpool_postgres::Pool;
use pbkdf2::{
    password_hash::{PasswordHash, PasswordVerifier},
    Pbkdf2,
};

// TODO clean up this struct
pub struct Account {
    pub id: i32,
    pub name: String,
    password: String,
    pin: String,
    pic: String,
    login_state: LoginState,
    last_login: Option<NaiveDateTime>,
    banned: bool,
    accepted_tos: bool,
}

#[derive(Debug)]
pub enum LoginError {
    AccountNotFound = 5,
    InvalidPassword = 0,
    AccountBanned = 3,
    AcceptTOS = 23,
    AlreadyLoggedIn = 7,
}

#[derive(PartialEq)]
enum LoginState {
    LoggedOut,
    Transitioning,
    LoggedIn,
    Error,
}

// TODO switch to using sync db?
pub async fn login(mut packet: Packet, client: &mut Client) {
    let name = packet.read_maple_string();
    let password = packet.read_maple_string();
    packet.advance(6);
    let hwid = packet.read_bytes(4);

    log::debug!(
        "Attempting to login: [name: {}, password: {}, hwid: {:?}]",
        name,
        password,
        hwid
    );

    // TODO check number of login attemps => not sure where to keep track

    let account = match get_account(&name, &client.pool).await {
        Some(account) => account,
        None => {
            client
                .send_packet(packets::login_failed(LoginError::AccountNotFound))
                .await;
            return;
        }
    };

    client.account = Some(account);

    if let Err(e) = validate_account(client.account.as_ref().unwrap(), password).await {
        client.send_packet(packets::login_failed(e)).await;
        return;
    }

    login_success(client).await;
}

pub async fn accept_tos(mut packet: Packet, client: &mut Client) {
    // Ok => 0x01, Cancel => 0x00
    let accepted = packet.read_byte();

    if accepted != 0x01 {
        log::info!("TOS was declined");
        return;
    }

    if client.account.is_none() {
        log::error!("Client's account is None");
        return;
    }

    let db = &client.pool.get().await.unwrap();

    if let Err(e) = db
        .query(
            "UPDATE accounts SET accepted_tos = true WHERE id = $1",
            &[&client.account.as_ref().unwrap().id],
        )
        .await
    {
        log::debug!("An error occurred while accepting tos: {}", e);
    }

    login_success(client).await;
}

async fn get_account(name: &String, pool: &Pool) -> Option<Account> {
    let db = pool.get().await.unwrap();
    let rows = match db
        .query(
            "SELECT id, name, password, pin, pic, login_state, last_login, banned, accepted_tos FROM accounts WHERE name = $1",
            &[&name],
        )
        .await
    {
        Ok(rows) => {
            if rows.len() == 0 {
                return None;
            }

            rows
        }
        Err(e) => {
            log::error!(
                "An error occurred while retrieving account information: {}",
                e
            );
            return None;
        }
    };

    let account = Account {
        id: rows[0].get(0),
        name: rows[0].get(1),
        password: rows[0].get(2),
        pin: rows[0].get(3),
        pic: rows[0].get(4),
        login_state: get_login_state(rows[0].get(5)),
        last_login: rows[0].get(6),
        banned: rows[0].get(7),
        accepted_tos: rows[0].get(8),
    };

    Some(account)
}

async fn validate_account(account: &Account, password: String) -> Result<(), LoginError> {
    if account.banned {
        return Err(LoginError::AccountBanned);
    }

    // TODO may need additional logic for login_state, should be fine for now
    if account.login_state != LoginState::LoggedOut {
        return Err(LoginError::AlreadyLoggedIn);
    }

    // if the account hasn't accepted tos, show the tos popup
    if !account.accepted_tos {
        return Err(LoginError::AcceptTOS);
    }

    // validate the entered password
    if let Err(e) = validate_password(account, password).await {
        return Err(e);
    }

    Ok(())
}

async fn validate_password(account: &Account, password: String) -> Result<(), LoginError> {
    // get the entered password's bytes
    let password = password.as_bytes();

    // get the account's hashed password
    let hash: &String = &account.password;
    let parsed_hash = PasswordHash::new(hash).unwrap();

    // check the entered password against the parsed hash
    if Pbkdf2.verify_password(password, &parsed_hash).is_err() {
        return Err(LoginError::InvalidPassword);
    }

    Ok(())
}

fn get_login_state(state: Option<i16>) -> LoginState {
    match state {
        Some(0) => LoginState::LoggedOut,
        Some(1) => LoginState::Transitioning,
        Some(2) => LoginState::LoggedOut,
        _ => LoginState::Error,
    }
}

async fn login_success(client: &mut Client) {
    // TODO update login state
    // TODO update last_loggedin

    client
        .send_packet(packets::login_success(client.account.as_ref().unwrap()))
        .await;
}