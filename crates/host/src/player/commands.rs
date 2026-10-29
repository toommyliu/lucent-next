use lucent_protocol::{bridge::to_json, command::Command, item::ItemContainer};
use serde::Serialize;
use serde_json::{Value as Json, json};

use super::Running;
use crate::bridge::{self, Error, Value};
use crate::projection::{
    Projection, commands,
    world::{MonsterLookup, monster_lookup},
};

impl Running {
    fn invoke_command(&mut self, name: &str, args: Vec<Value>) -> Result<Value, Error> {
        let result = bridge::invoke(&mut self.player.lock().unwrap(), name, args);
        self.drain_events();
        result
    }

    fn command_answer(&mut self, name: &str, args: Vec<Value>) -> Result<Json, Error> {
        to_json(self.invoke_command(name, args)?)
            .ok_or_else(|| commands::decode_error(name, "the callback returned undefined"))
    }

    fn commit_command<R: Serialize>(
        &mut self,
        commit: impl FnOnce(&mut Projection) -> Result<R, Error>,
    ) -> Result<String, Error> {
        self.drain_events();
        commit(&mut self.projection.write()).map(|output| serde_json::to_string(&output).unwrap())
    }

    pub(super) fn command(&mut self, command: Command) -> Result<String, Error> {
        match command {
            Command::RefreshContainer { container } => {
                let (_, callback) = commands::container_kind(container);
                let answer = self.command_answer(callback, vec![])?;
                self.commit_command(|projection| {
                    commands::refresh_container(projection, container, answer)
                })
            }
            Command::ConfirmShopClosed { shop_id } => {
                let args = shop_id
                    .into_iter()
                    .map(|id| Value::Number(id as f64))
                    .collect();
                let answer = self.command_answer("shops.isOpen", args)?;
                self.commit_command(|projection| commands::confirm_shop_closed(projection, answer))
            }
            Command::RejectDrop { selector } => {
                let id = self
                    .projection
                    .read()
                    .items
                    .resolve(ItemContainer::Drop, &selector)
                    .map(|item| item.data().item_id);
                let Some(id) = id else {
                    return Ok("false".into());
                };
                let answer = self.command_answer("drops.reject", vec![Value::Number(id as f64)])?;
                self.commit_command(|projection| commands::reject_drop(projection, id, answer))
            }
            Command::RefreshCells {} => {
                let loaded = commands::decode::<bool>(
                    "world.isLoaded",
                    self.command_answer("world.isLoaded", vec![])?,
                )?;
                if !loaded {
                    return self.commit_command(|projection| Ok(projection.world.cells()));
                }
                let answer = self.command_answer("world.getCells", vec![])?;
                self.commit_command(|projection| commands::refresh_cells(projection, answer))
            }
            Command::RefreshCellPads {} => {
                let loaded = commands::decode::<bool>(
                    "world.isLoaded",
                    self.command_answer("world.isLoaded", vec![])?,
                )?;
                if !loaded {
                    return self.commit_command(|projection| Ok(projection.world.cell_pads()));
                }
                let answer = self.command_answer("world.getCellPads", vec![])?;
                self.commit_command(|projection| commands::refresh_cell_pads(projection, answer))
            }
            Command::HydrateMonster { selector } => {
                if let Some(monster) = self.projection.read().world.resolve_monster(&selector) {
                    return Ok(serde_json::to_string(&monster.snapshot_data()).unwrap());
                }
                let selector = match monster_lookup(&selector) {
                    MonsterLookup::Id(id) => json!({"monsterMapId": id}),
                    MonsterLookup::Name(name) => json!({"name": name}),
                };
                let answer = self.command_answer(
                    "world.getMonster",
                    vec![lucent_protocol::bridge::from_json(selector)],
                )?;
                self.commit_command(|projection| commands::hydrate_monster(projection, answer))
            }
            Command::Login { username, password } => {
                let previous =
                    commands::install_login(&mut self.projection.write(), username, password);
                let Some(previous) = previous else {
                    return Ok("false".into());
                };
                let credentials = self.projection.read().auth.credentials();
                let result = self
                    .invoke_command(
                        "auth.login",
                        vec![
                            Value::String(credentials.username),
                            Value::String(credentials.password),
                        ],
                    )
                    .and_then(|_| self.commit_command(|_| Ok(true)));
                if result.is_err() {
                    self.projection.write().auth.set_credentials(previous);
                }
                result
            }
            Command::Logout {} => {
                let result = self.invoke_command("auth.logout", vec![]);
                let output = self.commit_command(|projection| {
                    projection.auth.clear();
                    Ok(())
                })?;
                result.map(|_| output)
            }
            Command::RefreshCredentials {} => {
                if !self
                    .projection
                    .read()
                    .current_readiness()
                    .missing
                    .is_empty()
                {
                    return Ok(
                        serde_json::to_string(&self.projection.read().auth.credentials()).unwrap(),
                    );
                }
                let username = self
                    .command_answer(
                        "flash.getGameObjectS",
                        vec![Value::String("loginInfo.strUsername".into())],
                    )
                    .ok();
                let password = self
                    .command_answer(
                        "flash.getGameObjectS",
                        vec![Value::String("loginInfo.strPassword".into())],
                    )
                    .ok();
                self.commit_command(|projection| {
                    commands::refresh_credentials(projection, username, password)
                })
            }
            Command::RefreshServers {} => {
                let answer = self.command_answer("auth.getServers", vec![]).ok();
                self.commit_command(|projection| commands::refresh_servers(projection, answer))
            }
            Command::RefreshLoggedIn {} => {
                let answer = self.command_answer("auth.isLoggedIn", vec![]).ok();
                self.commit_command(|projection| commands::refresh_logged_in(projection, answer))
            }
            Command::RefreshAcceptedQuests {} => {
                let answer = self.command_answer("quests.getAccepted", vec![])?;
                self.commit_command(|projection| {
                    commands::refresh_accepted_quests(projection, answer)
                })
            }
            Command::RefreshEquippedConsumable {} => {
                let answer = self.command_answer("combat.getConsumableSkillItem", vec![])?;
                self.commit_command(|projection| {
                    commands::refresh_equipped_consumable(projection, answer)
                })
            }
        }
    }
}
