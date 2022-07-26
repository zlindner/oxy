use crate::{
    character::{self, Character},
    client::{Client, LoginState},
    login::packets,
    net::packet::Packet,
    world::CapacityStatus,
};

pub async fn character_list(mut packet: Packet, client: &mut Client) {
    // not sure what this byte is for
    packet.advance(1);

    let world_id = packet.read_byte();

    let server = client.server.clone();
    let server = server.lock().await;
    let world = server.worlds.get(world_id as usize);

    // TODO add check to see if world.capacity_status is Full
    if world.is_none() {
        client
            .send_packet(packets::world_status(CapacityStatus::Full))
            .await;

        return;
    }

    let world = world.unwrap();

    let channel_id = packet.read_byte();
    let channel = world.channels.get(channel_id as usize);

    if channel.is_none() {
        client
            .send_packet(packets::world_status(CapacityStatus::Full))
            .await;

        return;
    }

    // TODO client.set_world(world)
    // TODO client.set_channel(channel)
    client.send_packet(packets::character_list()).await;
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

pub async fn validate_character_name(mut packet: Packet, client: &mut Client) {
    let name = packet.read_maple_string();

    // looks like the client has it's own "banned name" list, so we can skip implementing that for now
    let valid = !is_name_taken(&name, &client.pool).await;

    client
        .send_packet(packets::character_name(&name, valid))
        .await;
}

pub async fn create_character(mut packet: Packet, client: &mut Client) {
    let name = packet.read_maple_string();
    let job = packet.read_int();

    let face = packet.read_int();
    let hair = packet.read_int();
    let hair_colour = packet.read_int();
    let skin_colour = packet.read_int();

    let top = packet.read_int();
    let bottom = packet.read_int();
    let shoes = packet.read_int();
    let weapon = packet.read_int();
    let gender = packet.read_byte();

    // TODO check if face, hair, top, bottom, shoes, weapon are valid => match the correct ids for starter gear
    // this is done to prevent packet editing during character creation
    // if invalid, disconnect the client

    // TODO check clients available character slots
    let style = character::Style::new(skin_colour, gender, hair + hair_colour, face);
    let mut character = Character::new(0, 0, name, style);
    log::debug!("character: {:?}", character);

    let id = save_new_character(&character, &client.pool).await;

    if id.is_none() {
        return;
    }

    // update the character id to what is returned by db
    character.id = id.unwrap();

    client
        .send_packet(packets::create_character(&character))
        .await;
}

async fn is_name_taken(name: &str, pool: &Pool) -> bool {
    let db = pool.get().await.unwrap();

    let query = db
        .query("SELECT id FROM characters WHERE name = $1", &[&name])
        .await;

    if query.is_err() {
        return true;
    }

    // no rows found => name isn't taken
    let rows = query.unwrap();
    rows.len() > 0
}

// TODO look into creating a queries.rs or something to clean up this file
async fn save_new_character(character: &Character, pool: &Pool) -> Option<i32> {
    let db = pool.get().await.unwrap();

    match db.query("INSERT INTO characters (account_id, world_id, name, level, str, dex, luk, int, hp, mp, max_hp, max_mp, mesos, job, skin_colour, gender, hair, face, ap, sp, map, spawn_point, gm) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23) RETURNING id", &[&character.account_id, &character.world_id, &character.name, &(character.stats.level as i16), &character.stats.str, &character.stats.dex, &character.stats.luk, &character.stats.int, &character.stats.hp, &character.stats.mp, &character.stats.max_hp, &character.stats.max_mp, &character.stats.mesos, &character.job, &character.style.skin_colour, &(character.style.gender as i16), &character.style.hair, &character.style.face, &character.stats.ap, &character.stats.sp, &character.map, &character.spawn_point, &(character.gm as i16)]).await {
        Ok(rows) => {
            if rows.len() == 0 {
                return None;
            }

            return Some(rows[0].get(0))
        }
        Err(e) => {
            log::error!("An error occurred while saving character to db: {}", e);
            return None;
        }
    };
}
