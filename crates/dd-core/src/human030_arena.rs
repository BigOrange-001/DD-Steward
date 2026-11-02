//! Human-owned frozen public arena. Never given to either adviser.
use super::*;
use serde_json::json;
pub struct Arena {
    pub input: PublicBattle,
    battle: Battle,
    pub id: String,
    pub game: GameData,
}
impl Arena {
    pub fn new(id: &str, input: PublicBattle, seed: u64) -> Result<Self> {
        let game = GameData::load_with_content(&dd_data::game_dir(), input.content.clone())?;
        let defs = Definitions::project029(&game, &input, true)?;
        let input = complete_historical(&input, &defs)?;
        let defs = Definitions::project029(&game, &input, true)?;
        let mut model = reconstruct(&input, &defs, 0)?.sample(seed)?;
        model.battle.scenario.round_limit = 1000;
        let mut result = Self {
            input,
            battle: model.battle,
            id: id.into(),
            game,
        };
        result.settle()?;
        Ok(result)
    }
    pub fn observation(&self) -> Observation {
        self.battle.observation()
    }
    fn settle(&mut self) -> Result<()> {
        for _ in 0..100000 {
            let o = self.observation();
            if o.outcome.is_some()
                || o.player_selection
                || o.actor.is_some_and(|id| {
                    o.units
                        .iter()
                        .any(|u| u.id == id && u.hero && u.controlled.is_none())
                })
            {
                return Ok(());
            }
            let a = self.battle.monster_action()?;
            self.battle.step_settled(a)?;
            if let Some(e) = self.battle.lifecycle.failure.take() {
                return Err(e);
            }
        }
        Err("030 arena automatic transition bound".into())
    }
    pub fn act(&mut self, action: Action) -> Result<()> {
        // Transaction for real execution, not a policy simulation.
        let saved = self.battle.clone();
        let result = (|| {
            execution025::with_options(self.input.mechanisms, || {
                execution024b::run(self.input.single_target_fallback, || {
                    self.battle.step_settled(action)?;
                    if let Some(e) = self.battle.lifecycle.failure.take() {
                        return Err(e);
                    }
                    self.settle()
                })
                .map(|r| r.0)
            })
        })();
        if result.is_err() {
            self.battle = saved;
        }
        result
    }
    pub fn save(&self) -> Result<Value> {
        Ok(json!({"id":self.id,"input":self.input,"battle":self.battle.save_json()?}))
    }
    pub fn restore(v: &Value) -> Result<Self> {
        let input: PublicBattle =
            serde_json::from_value(v["input"].clone()).map_err(|e| e.to_string())?;
        let game = GameData::load_with_content(&dd_data::game_dir(), input.content.clone())?;
        let battle = Battle::load_json_with_content(
            v["battle"].as_str().ok_or("030 absent native arena")?,
            &input.content,
        )?;
        Ok(Self {
            id: v["id"].as_str().ok_or("030 arena id")?.into(),
            input,
            game,
            battle,
        })
    }
    pub fn native(&self) -> Result<String> {
        self.battle.save_json()
    }
    }
