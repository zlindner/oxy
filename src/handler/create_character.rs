use crate::{
    character::{Character, Rank, Stats, Style},
    client::Client,
    login::{packets, queries},
    net::packet::Packet,
    Result,
};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::collections::HashSet;

static STARTER_WEAPONS: Lazy<HashSet<i32>> = Lazy::new(|| {
    [
        1302000, // sword
        1312004, // hand axe
        1322005, // wooden club
        1442079, // basic polearm
    ]
    .into_iter()
    .collect()
});

static STARTER_TOPS: Lazy<HashSet<i32>> = Lazy::new(|| {
    [
        1040002, // white undershirt
        1040006, // undershirt
        1040010, // grey t-shirt
        1041002, // white tubetop
        1041006, // yellow t-shirt
        1041010, // green t-shirt
        1041011, // red striped top
        1042167, // simple warrior top
    ]
    .into_iter()
    .collect()
});

static STARTER_BOTTOMS: Lazy<HashSet<i32>> = Lazy::new(|| {
    [
        1060002, // blue jean shorts
        1060006, // brown cotton shorts
        1061002, // red miniskirt
        1061008, // indigo miniskirt
        1062115, // simple warrior pants
    ]
    .into_iter()
    .collect()
});

static STARTER_SHOES: Lazy<HashSet<i32>> = Lazy::new(|| {
    [
        1072001, // red rubber boots
        1072005, // leather sandals
        1072037, // yellow rubber boots
        1072038, // blue rubber boots
        1072383, // average musashi shoes
    ]
    .into_iter()
    .collect()
});

static STARTER_HAIR: Lazy<HashSet<i32>> = Lazy::new(|| {
    [
        30000, // toben
        30010, // zeta
        30020, // rebel
        30030, // buzz
        31000, // sammy
        31040, // edgie
        31050, // connie
    ]
    .into_iter()
    .collect()
});

static STARTER_FACE: Lazy<HashSet<i32>> = Lazy::new(|| {
    [
        20000, // motivated look (m)
        20001, // perplexed stare
        20002, // leisure look (m)
        21000, // motiviated look (f)
        21001, // fearful stare (m)
        21002, // leisure look (f)
        21201, // fearful stare (f)
        20401, // perplexed stare hazel
        20402, // leisure look hazel
        21700, // motivated look amethyst
        20100, // motivated look blue
    ]
    .into_iter()
    .collect()
});

#[derive(Debug)]
pub struct CreateCharacter {
    name: String,
    job: i32,
    face: i32,
    hair: i32,
    hair_colour: i32,
    skin_colour: i32,
    top: i32,
    bottom: i32,
    shoes: i32,
    weapon: i32,
    gender: u8,
}

impl CreateCharacter {
    pub fn new(mut packet: Packet) -> Self {
        Self {
            name: packet.read_string(),
            job: packet.read_int(),
            face: packet.read_int(),
            hair: packet.read_int(),
            hair_colour: packet.read_int(),
            skin_colour: packet.read_int(),
            top: packet.read_int(),
            bottom: packet.read_int(),
            shoes: packet.read_int(),
            weapon: packet.read_int(),
            gender: packet.read_byte(),
        }
    }

    pub async fn handle(&self, client: &mut Client) -> Result<()> {
        // character has invalid equipment (via packet editing), disconnect them
        if !STARTER_WEAPONS.contains(&self.weapon)
            || !STARTER_TOPS.contains(&self.top)
            || !STARTER_BOTTOMS.contains(&self.bottom)
            || !STARTER_SHOES.contains(&self.shoes)
            || !STARTER_HAIR.contains(&self.hair)
            || !STARTER_FACE.contains(&self.face)
        {
            client.disconnect().await?;
            return Ok(());
        }

        // TODO check job

        // beginner
        // job: beginner => 0
        // map: mushroom town => 10000
        // give item: beginners guide => 4161001

        // TODO check to make sure client has available character slots
        // TODO check if character name is valid

        let style = Style {
            skin_colour: self.skin_colour,
            gender: self.gender,
            hair: self.hair,
            face: self.face,
        };

        let inventory = DashMap::new();
        inventory.insert(-5, self.top);
        inventory.insert(-6, self.bottom);
        inventory.insert(-7, self.shoes);
        inventory.insert(-11, self.weapon);

        let character = Character {
            id: 0,
            account_id: client.id.unwrap(),
            world_id: client.world_id.unwrap(),
            name: self.name.clone(),
            stats: Stats::default(),
            job: self.job,
            style: style,
            map: 10000,
            spawn_point: 10000,
            gm: 0,
            rank: Rank::default(),
            pets: Vec::new(),
            inventory: inventory,
        };

        queries::create_character(&character, &client.db).await?;
        // TODO update keymap table
        // TODO update inventoryitems, inventoryequipment table
        // TODO update skills table

        client
            .connection
            .write_packet(packets::new_character(&character))
            .await?;

        Ok(())
    }
}