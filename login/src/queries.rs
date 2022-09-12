use oxide_core::{Db, Result};

pub async fn update_login_state(id: i32, login_state: i32, db: &Db) -> Result<()> {
    sqlx::query(
        "UPDATE accounts \
        SET login_state = $1, last_login = CURRENT_TIMESTAMP \
        WHERE id = $2",
    )
    .bind(login_state)
    .bind(id)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn update_pin(id: i32, pin: &String, db: &Db) -> Result<()> {
    sqlx::query(
        "UPDATE accounts \
        SET pin = $1 \
        WHERE id = $2",
    )
    .bind(pin)
    .bind(id)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn logout_all(db: &Db) -> Result<()> {
    sqlx::query(
        "UPDATE accounts \
        SET login_state = 0, last_login = CURRENT_TIMESTAMP \
        WHERE login_state != 0",
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn clear_sessions(db: &Db) -> Result<()> {
    sqlx::query("DELETE FROM sessions").execute(db).await?;
    Ok(())
}
