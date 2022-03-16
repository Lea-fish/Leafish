use crate::format::Color;
use serde::{
    de::{MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use std::fmt;

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum ComponentData {
    Chat(Chat),
    Str(String),
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClickEvent {
    pub action: String,
    pub value: String,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Text {
    pub text: String,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Contents {
    pub id: Option<String>,
    pub name: Option<Text>,
    pub count: Option<usize>,
    pub r#type: Option<String>,
    pub text: Option<String>,
    pub extra: Option<Vec<ComponentData>>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct HoverEvent {
    pub action: String,
    pub contents: Option<Contents>,
    pub value: Option<Text>,
    pub r#type: Option<String>,
}

#[derive(Default, Clone, PartialEq)]
pub struct Chat {
    pub translate: Option<String>,

    pub color: Option<Color>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underlined: Option<bool>,
    pub strikethrough: Option<bool>,
    pub obfuscated: Option<bool>,

    pub click_event: Option<ClickEvent>,
    pub hover_event: Option<HoverEvent>,
    pub insertion: Option<String>,
    pub text: Option<String>,

    pub extra: Option<Vec<ComponentData>>,
    pub with: Vec<ComponentData>,
}

/// Can be deserialized from a single element, which will produce one section,
/// or from an array of sections.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct ChatSections {
    pub sections: Vec<Chat>,
}

struct ChatVisitor;

/// This needs to be custom, so that the `ChatSections` deserializer can access
/// the `ChatVisitor`.
impl<'de> Deserialize<'de> for Chat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(ChatVisitor)
    }
}

impl<'de> Visitor<'de> for ChatVisitor {
    type Value = Chat;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a chat section")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut chat = Chat::default();
        // Need to deserialize into a String to make this work for all 3 types of strings
        while let Some(key) = access.next_key::<String>()? {
            match key.as_str() {
                "translate" => chat.translate = Some(access.next_value()?),

                "color" => chat.color = Some(access.next_value()?),
                "bold" => chat.bold = Some(access.next_value()?),
                "italic" => chat.italic = Some(access.next_value()?),
                "underlined" => chat.underlined = Some(access.next_value()?),
                "strikethrough" => chat.strikethrough = Some(access.next_value()?),
                "obfuscated" => chat.obfuscated = Some(access.next_value()?),

                "clickEvent" => chat.click_event = Some(access.next_value()?),
                "hoverEvent" => chat.hover_event = Some(access.next_value()?),
                "insertion" => chat.insertion = Some(access.next_value()?),
                "text" => chat.text = Some(access.next_value()?),

                "extra" => chat.extra = Some(access.next_value()?),
                "with" => chat.with = access.next_value()?,
                _ => {}
            }
        }
        Ok(chat)
    }
}
impl<'de> Deserialize<'de> for ChatSections {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ChatSectionsVisitor;

        impl<'de> Visitor<'de> for ChatSectionsVisitor {
            type Value = ChatSections;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a chat section")
            }

            fn visit_map<M>(self, access: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                Ok(ChatSections {
                    sections: vec![ChatVisitor.visit_map(access)?],
                })
            }

            fn visit_seq<A>(self, mut access: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut sections = Vec::with_capacity(access.size_hint().unwrap_or(0));
                while let Some(section) = access.next_element()? {
                    sections.push(section);
                }
                Ok(ChatSections { sections })
            }
        }

        deserializer.deserialize_any(ChatSectionsVisitor)
    }
}

impl std::fmt::Debug for Chat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("Chat");
        if self.translate.is_some() {
            dbg.field("translate", &self.translate);
        }
        if self.color.is_some() {
            dbg.field("color", &self.color);
        }
        if self.bold.is_some() {
            dbg.field("bold", &self.bold);
        }
        if self.italic.is_some() {
            dbg.field("italic", &self.italic);
        }
        if self.underlined.is_some() {
            dbg.field("underlined", &self.underlined);
        }
        if self.strikethrough.is_some() {
            dbg.field("strikethrough", &self.strikethrough);
        }
        if self.obfuscated.is_some() {
            dbg.field("obfuscated", &self.obfuscated);
        }
        if self.click_event.is_some() {
            dbg.field("click_event", &self.click_event);
        }
        if self.hover_event.is_some() {
            dbg.field("hover_event", &self.hover_event);
        }
        if self.insertion.is_some() {
            dbg.field("insertion", &self.insertion);
        }
        if self.text.is_some() {
            dbg.field("text", &self.text);
        }
        if self.extra.is_some() {
            dbg.field("extra", &self.extra);
        }
        if self.with.len() > 0 {
            dbg.field("with", &self.with);
        }
        dbg.finish_non_exhaustive()
    }
}

// TODO: Add multiple language support.
pub fn translate(str: &str) -> &str {
    match str {
            "commands.message.sameTarget" => "You can't send a private message to yourself!",
            "commands.op.success" => "Made {} a server operator",                       // minecraft 1.12.2 - "Opped {}"
            "commands.op.failed" => "Nothing changed. The player alreadys an operator", // Does not exits in minecraf 1.12.2
            "commands.deop.success" => "Made {} no longer a server operator",           // minecraft 1.12.2 - "De-opped {}"
            "commands.deop.failed" => "Nothing changed. The player is not an operator", // minecraft 1.12.2 - "Could not de-op {}"
            "commands.time.set" => "Set the time to {}",
            "commands.time.query" => "Time is {}",
            "commands.time.add" => "Added {} to the time",
            "commands.tp.success.coordinates" => "Teleported {} to {}, {}, {}",
            "commands.teleport.success.location.single" => "Teleported {} to {}, {}, {}",
            "commands.tp.success" => "Teleported {} to {}",
            "commands.teleport.success.entity.single" => "Teleported {} to {}",
            "commands.players.list" => "There are {}/{} players online:",
            "commands.fill.success" => "{} blocks filled",
            "commands.function.unknown" => "Unknown function '{}'",
            "commands.gamerule.success" => "Game rule {} has been updated to {}",
            "commands.kick.success.reason" => "Kicked {} from the game: {}",
            "commands.kick.success" => "Kicked {} from the game",
            "commands.kill.successful" => "Killed {}",
            "commands.locate.success" => "Located {} at {} (y?) {}",
            "commands.locate.failure" => "Unable to locate any {} feature",
            "commands.reload.success" => "Successfully reloaded loot tables, advancements and functions",
            "commands.recipe.give.success.one" => "Successfully given {} the recipe for {}",
            "commands.recipe.alreadyHave" => "Player {} already has a recipe for {}",
            "commands.seed.success" => "Seed: {}",
            "commands.setblock.success" => "Block placed",
            "commands.clear.success" => "Cleared the inventory of {}, removing {} items",
            "commands.weather.clear" => "Changing to clear weather",
            "commands.weather.set.clear" => "Set the weather to clear",
            "commands.weather.rain" => "Changing to rainy weather",
            "commands.weather.thunder" => "Changing to rain and thunder",
            "commands.setidletimeout.success" => "Successfully set the idle timeout to {} minutes.",
            "commands.setworldspawn.success" => "Set the world spawn point to ({}, {}, {})",
            "commands.spreadplayers.success.players" => "Successfully spread {} player(s) around {} {}",
            "commands.spreadplayers.spreading.players" => "Spreading {} player(s) {} blocks around {}, {} (min {} blocks apart)",
            "commands.spreadplayers.info.players" => "(Average distance between players is {} blocks apart after {} iterations)",
            "commands.summon.success" => "Object successfully summoned",
            "commands.stop.start" => "Server stopping...",
            "commands.stopsound.success.soundSource" => "Stopped source '{}' for {}",
            "commands.testfor.success" => "Found {}",
            "commands.downfall.success" => "Toggled downfall",

            "commands.worldborder.get.success" => "World border is currently {} blocks wide",
            "commands.worldborder.setSlowly.grow.success" => "Growing world border to {} blocks wide (up from {} blocks) over {} seconds",
            "commands.worldborder.setSlowly.shrink.success" => "Shrinking world border to {} blocks wide (down from {} blocks) over {} seconds",
            "commands.worldborder.center.success" => "Set world border center to {}, {}",
            "commands.worldborder.damage.buffer.success" => "Set world border damage buffer to {} blocks (from {} blocks)",
            "commands.worldborder.damage.amount.success" => "Set world border damage amount to {} per block (from {} per block}",
            "commands.worldborder.warning.time.success" => "Set world border warning to {} seconds away (from {} seconds)",
            "commands.worldborder.warning.distance.success" => "Set world border warning to {} blocks away (from {} blocks)",

            "commands.whitelist.reloaded" => "Reload the whitelist",
            "commands.whitelist.add.success" => "Added {} to the whitelist",
            "commands.whitelist.remove.success" => "Removed {} from the whitelist",
            "commands.whitelist.list" => "There are {} whitelisted players: {}",            // This would not work right on minecraft 1.12.2. Format for 1.12.2 "There are {} (out of {} seen) whitelisted players:"
            "commands.whitelist.none" => "There are no whitelisted players",
            "commands.whitelist.enabled" => "Whitelist is now turned on",                   // 1.12.2 format Turned on the whitelist
            "commands.whitelist.disabled" => "Whitelist is now turned off",                 // 1.12.2 format "Turned off the whitelist"


            "commands.testforblock.success" => "Successfully found the block at {}, {}, {}",
            "commands.testforblock.failed.tile" => "The block at {}, {}, {} is {} (expected: {})",
            "commands.compare.success" => "{} blocks compared",
            "commands.compare.failed" => "Source and destination are not identical",

            "commands.save.start" => "Saving...",
            "commands.save.success" => "Saved the world",
            "commands.save.enabled" => "Turned on world auto-saving",
            "commands.save.disabled" => "Turned off world auto-saving",



            "commands.scoreboard.players.usage" => "/scoreboard players <set:add:remove:reset:list:enable:test:operation:tag>",
            "commands.scoreboard.players.list.empty" => "There are no tracked players on the scoreboard",
            "commands.scoreboard.players.remove.usage" => "/scoreboard players remove <player> <objective>",
            "commands.scoreboard.players.add.usage" => "/scoreboard players add <player> <objective>",
            "commands.scoreboard.players.operation.usage" => "/scoreboard players operation <targetName> <targetObjective> <operation> <selector> <objective>",
            "commands.scoreboard.players.reset.usage" => "/scoreboard players reset <player> <objective>",
            "commands.scoreboard.players.enable.usage" => "/scoreboard players enable <player> <trigger>",
            "commands.scoreboard.players.set.usage" => "/scoreboard players set <player> <objective> <score> [dataTag]",
            "commands.scoreboard.players.test.usage" => "/scoreboard players test <player> <objective> <min> [max]",

            "commands.scoreboard.objectives.usage" => "/scoreboard objectives <list:add:remove:setdisplay> ...",
            "commands.scoreboard.objectives.list.empty" => "There are no objectives on the scoreboard",
            "commands.scoreboard.objectives.remove.usage" => "/scoreboard objectives remove <name>",
            "commands.scoreboard.objectives.setdisplay.usage" => "/scoreboard objectives setdisplay <slot> [objective]",
            "commands.scoreboard.objectives.add.usage" => "/scoreboard objectives add <name> <criteriaType> [display name ...]",

            "commands.scoreboard.teams.usage" => "/scoreboard teams <list:add:remove:empty:join:leave:option> ...",
            "commands.scoreboard.teams.list.empty" => "There are no teams registered on the scoreboard",
            "commands.scoreboard.teams.option.usage" => "/scoreboard teams option <team> <friendlyfire:color:seeFriendlyInvisibles:nameTagVisibility:deathMessageVisibility:collisionRule> <value>",
            "commands.scoreboard.teams.add.usage" => "/scoreboard teams add <name> [display name ...]",
            "commands.scoreboard.teams.add.success" => "Added new team '{}' successfully",
            "commands.scoreboard.teams.add.alreadyExists" => "A team with the name '{}' already exists",
            "commands.scoreboard.teams.remove.usage" => "/scoreboard teams remove <name>",
            "commands.scoreboard.teams.join.usage" => "/scoreboard teams join <team> [player]",
            "commands.scoreboard.teams.join.success" => "Added {} player(s) to team {}: {}",
            "commands.scoreboard.teams.leave.success" => "Removed {} player(s) from their teams: {}",
            "commands.scoreboard.teams.leave.failure" => "Could not remove {} player(s) from their teams: {}",
            "commands.scoreboard.teams.list.count" => "Showing {} teams on the scoreboard:",
            "commands.scoreboard.teams.list.entry" => "- {}: '{}' has 1 players",
            "commands.scoreboard.teams.list.player.count" => "Showing {} player(s) in team {}:",

            "commands.replaceitem.noContainer" => "Block at {}, {}, {} is not a container",
            "commands.generic.parameter.invalid" => "'{}' is not a valid parameter",
            "commands.replaceitem.success" => "Replaced slot {} with {} * {}",

            "commands.unban.success" => "Unbanned player {}",
            "commands.unban.failed" => "Could not unban player {}",
            "commands.ban.success" => "Banned player {}",


            "commands.xp.success" => "Given {} experience to {}",
            "commands.xp.success.levels" => "Given {} levels to {}",
            "commands.xp.success.negative.levels" => "Taken {} levels from {}",
            "commands.xp.failure.widthdrawXp" => "Cannot give player negative experience points",

            "commands.experience.add.points.success.single" => "Gave {} experience points to {}",
            "commands.experience.add.points.success.multiple" => "Gave {} experience points to {} players",
            "commands.experience.set.points.success.single" => "Set {} experience points on {}",
            "commands.experience.set.points.success.multiple" => "Set {} experience points on {} players",
            "commands.experience.query.points" => "{} has {} experience points",

            "commands.experience.add.levels.success.single" => "Gave {} experience levels to {}",
            "commands.experience.add.levels.success.multiple" => "Gave {} experience levels to {} players",
            "commands.experience.set.levels.success.single" => "Set {} experience levels on {}",
            "commands.experience.set.levels.success.multiple" => "Set {} experience levels on {} players",
            "commands.experience.query.levels" => "{} has {} experience levels",


            "commands.enchant.success" => "Enchanting succeeded",
            "commands.enchant.noItem" => "The target doesn't hold an item",

            "commands.debug.start" => "Started debug profiling",
            "commands.debug.stop" => "Stopped debug profiling after {} seconds ({} ticks)",

            "commands.difficulty.success" => "Set game difficulty to {}",
            "options.difficulty.easy" => "Easy",
            "options.difficulty.normal" => "Normal",
            "options.difficulty.hard" => "Hard",


            "commands.advancement.revoke.through.success" => "Revoked '{}', all ancestors and all descendants ({} total revoked) from {}",
            "commands.advancement.revoke.everything.success" => "Revoked every advancement ({} total revoked) from {}",
            "commands.advancement.revoke.from.success" => "Revoked '{}' and all descendants ({} total revoked) from {}",
            "commands.advancement.revoke.from.failed" => "Couln't revoke the advancement '{}' or its descendants from {} because they haven't started any",
            "commands.advancement.revoke.only.success" => "Revoked the entire advancement '{}' from {}",
            "commands.advancement.revoke.only.failed" => "Couldn't revoke the advancement '{}' from {} because they haven't started it",

            "commands.advancement.grant.only.success" => "Granted the entire advancement '{}' to {}",
            "commands.advancement.grant.only.failed" => "Couldn't grant the advancement '{}' to {} because they already have it",
            "commands.advancement.grant.from.success" => "Granted '{}' and all descendants ({} total granted) to {}",
            "commands.advancement.grant.from.failed" => "Couldn't grant the advancement '{}' or its descendants to {} because they already have them all",
            "commands.advancement.grant.everything.success" => "Granted every advancement ({} total granted) to {}",
            "commands.advancement.grant.everything.failed" => "Couldn't grant any advancements to {} because they already have them all",

            "commands.advancement.grant.many.to.one.success" => "Granted {} advancements to {}",
            "commands.advancement.grant.one.to.many.success" => "Granted the advancement {} to {} players",
            "commands.advancement.advancementNotFound" => "No advancement was found by the name '{}'",


            "commands.datapack.list.available.none" => "There are no more data packs available",
            "commands.datapack.list.enabled.success" => "There are {} data packs enabled: {}",
            "pack.nameAndSource" => "{} {}",
            "pack.source.builtin" => "built-in",
            "commands.execute.failed" => "Failed to execute '{}' as {}",
            "commands.generic.player.unspecified" => "You must specify which player you wish to perform this action on.",
            "commands.message.display.outgoing" => "You whisper to {}: {}",
            "commands.message.display.incoming" => "{} whispers to you: {}",
            "commands.generic.usage" => "Usage: {}",
            "commands.generic.notFound" => "Unknown command. Try /help for a list of commands",
            "command.unknown.command" => "Unknown or incomplete command. see below for error",
            "command.unknown.argument" => "Incorrect argument for command",
            "command.context.here" => "<--[HERE]",
            "commands.generic.entity.notFound" => "Entity '{}' cannot be found",
            "argument.entity.notfound.entity" => "No entity was found",
            "commands.generic.player.notFound" => "Player '{}' cannot be found",
            "argument.entity.notfound.player" => "No player was found",
            "argument.player.toomany" => "Only one player is allowed, but the provided selector allows more than one",
            "argument.player.entities" => "Only players may be affected by this command. provided selector includes entities",
            "argument.item.id.invalid" => "Unknown item '{}'",
            "argument.component.invalid" => "Invalid chat component: {}",
            "commands.generic.num.invalid" => "'{}' is not a valid number",
            "commands.generic.boolean.invalid" => "'{}' is not true or false",
            "commands.generic.help" => "Usage: /help [page:command name]",
            "commands.generic.num.tooBig" => "The number you have entered ({}) is too big, it must be at most {}",
            "commands.generic.num.tooSmall" => "The number you have entered ({}) is too small, it must be at least {}",

            "commands.give.success" => "Given {} * {} to {}",
            "commands.give.success.single" => "Gave {} {} to {}",
            "commands.give.item.notFound" => "There is no such item with name: {}",
            "commands.give.tagError" => "Data tag parsing failed: {}",
            "commands.give.block.notFound" => "There is no such block with name {}",

            "multiplayer.player.joined" => "{} joined the game",
            "multiplayer.player.left" => "{} left the game",
            "multiplayer.disconnect.kicked" => "Kicked by an operator.",
            "multiplayer.disconnect.server_shutdown" => "Server closed",

            "chat.type.admin" => "[{}: {}]",
            "chat.type.text" => "<{}> {}",
            "chat.type.announcement" => "[{}] {}",
            "chat.type.emote" => "* {} {}",
            "chat.square_brackets" => "[{}]",

            "chat.type.advancement.task" => "{} has made the advancement {}",
            "chat.type.advancement.challenge" => "{} has completed the challenge {}",
            "chat.type.advancement.goal" => "{} has reached the goal {}",
            "commands.tellraw.jsonException" => "Invalid json: {}",

            "gameMode.changed" => "Your game mode has been updated to {}",
            "commands.gamemode.success.self" => "Set own game mode to {}",
            "commands.gamemode.success.other" => "Set {}'s game mode to {}",
            "gameMode.creative" => "Creative Mode",
            "gameMode.adventure" => "Adventure Mode",
            "gameMode.survival" => "Survival Mode",
            "gameMode.spectator" => "Spectator Mode",

            "death.attack.mob" => "{} was slain by {}",
            "death.attack.arrow" => "{} was shot by {}",
            "death.attack.player" => "{} was slain by {}",
            "death.attack.explosion.player" => "{} was blown up by {}",
            "death.attack.lava" => "{} tried to swim in lava",
            "death.attack.drowned" => "{} drowned",
            "death.attack.outOfWorld" => "{} fell out of the world",
            "death.fell.accident.generic" => "{} fell from a high place",


            "advancements.adventure.kill_a_mob.title" => "Monster Hunter",
            "advancements.adventure.shoot_arrow.title" => "Take Aim",
            "advancements.adventure.trade.title" => "What a Deal!",

            "advancements.nether.obtain_blaze_rod.title" => "Into Fire",
            "advancements.nether.find_fortress.title" => "A Terrible Fortress",
            "advancements.nether.uneasy_alliance.title" => "Unesay Alliance",

            "advancements.husbandry.breed_an_animal.title" => "The Parrots and the Bats",

            "advancements.story.cure_zombie_villager.title" => "Zombie Doctor",
            "advancements.story.upgrade_tools.title" => "Getting an Upgrade",
            "advancements.story.mine_diamond.title" => "Diamonds!",
            "advancements.story.smelt_iron.title" => "Acquire Hardware",
            "advancements.story.enter_the_end.title" => "The End?",
            "advancements.story.form_obsidian.title" => "Ice Bucket Challenge",
            "advancements.story.enter_the_nether.title" => "We Need to Go Deeper",

            "advancements.end.enter_end_gateway.title" => "The City at the End of the Game",
            "advancements.end.find_end_city.title" => "Remote Getaway",
            "advancements.end.respawn_dragon.title" => "The End... Again...",


            "commands.help.header" => "--- Showing help page {} of {} (/help <page>) ---",
            "commands.help.footer" => "Tip Use the <tab> key while typing a command to auto-complete the command or its arguments",
            "commands.help.failed" => "Unknown command or insufficient permissions",
            "commands.advancement.usage" => "/advancement <grant:revoke:test> <player>",
            "commands.ban.usage" => "/ban <name> [reason ...]",
            "commands.banip.usage" => "/ban-ip <address:name> [reason ...]",
            "commands.unbanip.success" => "Unbanned IP address {}",
            "commands.unbanip.invalid" => "You have entered an invalid IP address",
            "commands.banip.success" => "Banned IP address {}",
            "commands.banlist.usage" => "/banlist [ips:players]",
            "commands.banlist.players" => "There are {} total banned players:",
            "commands.banlist.ips" => "There are {} total banned IP addresses:",

            "commands.title.success" => "Title command successfully executed",

            "commands.blockdata.usage" => "/blockdata <x> <y> <z> <dataTag>",
            "commands.clear.usage" => "/clear [player][item] [data] [maxCount] [dataTag]",
            "commands.clone.usage" => "/clone <x1> <y1> <z1> <x2> <y2> <z2> [maskMode][cloneMode]",
            "commands.debug.usage" => "/debug <start:stop>",
            "commands.defaultgamemode.usage" => "/defaultgamemode <mode>",
            "commands.deop.usage" => "/deop <player>",
            "commands.difficulty.usage" => "/difficulty <new difficulty>",
            "commands.effect.usage" => "/effect <player> <effect> [seconds] [amplifier] [hideParticles] OR /effect <player> clear",
            "commands.enchant.usage" => "/enchant <player> <enchantment ID> [level]",
            "commands.entitydata.usage" => "/entitydata <entity> <dataTag>",
            "commands.execute.usage" => "/execute <entity> <x> <y> <z> <command> OR /execute <entity> <x> <y> <z> detect <x> <y> <z> <block> <dataValue:-1:state:*> <command>",
            "commands.fill.usage" => "/fill <x1> <y1> <z1> <x2> <y2> <z2> <block> [dataValue:state] [oldBlockHandling [dataTag]]",
            "commands.function.usage" => "/function <name> [if <selector>:unless <selector>]",
            "commands.gamemode.usage" => "/gamemode <mode> <player>",
            "commands.gamerule.usage" => "/gamerule <rule name> [value]",
            "commands.give.usage" => "/give <player> <item> [amount] [data] [dataTag]",
            "commands.help.usage" => "/help [page:command name]",
            "commands.kick.usage" => "/kick <player> [reason ...]",
            "commands.kill.usage" => "/kill [player:entity]",
            "commands.players.usage" => "/list",
            "commands.locate.usage" => "/locate <feature>",
            "commands.me.usage" => "/me <action ...>",
            "commands.op.usage" => "/op <player>",
            "commands.unban.usage" => "/pardon <name>",
            "commands.unbanip.usage" => "/pardon-ip <address>",
            "commands.particle.usage" => "/particle <name> <x> <y> <z> <xd> <yd> zd> <speed> [count] [mode] [player] [params]",
            "commands.playsound.usage" => "/playsound <sound> <source> <player> [x] [y] [z] [volume] [pitch] [minimumVolume]",
            "commands.recipe.usage" => "/recipe <give:take> [player] <name:*>",
            "commands.reload.usage" => "/reload",
            "commands.replaceitem.usage" => "/replaceitem <entity:block> ...",
            "commands.save.usage" => "/save-all [flush]",
            "commands.save-off.usage" => "/save-off",
            "commands.save-on.usage" => "/save-on",
            "commands.say.usage" => "/say <message ...>",
            "commands.scoreboard.usage" => "/scoreboard <objectives:players:teams> ...",
            "commands.seed.usage" => "/seed",
            "commands.setblock.usage" => "/setblock <x> <y> <z> <block> [dataValue:state] [oldBlockHandling] [dataTag]",
            "commands.setidletimeout.usage" => "/setidletimeout <minutes>",
            "commands.setworldspawn.usage" => "/setworldspawn [<x> <y> <z>]",
            "commands.spawnpoint.usage" => "/spawnpoint [player] [<x> <y> <z>]",
            "commands.spreadplayers.usage" => "/spreadplayers <x> <z> <spreadDistance> <maxRange>",
            "commands.stats.usage" => "/stats <entity:block> ...",
            "commands.stop.usage" => "/stop",
            "commands.stopsound.usage" => "/stopsound <player> [source] [sound]",
            "commands.summon.usage" => "/summon <entityname> [x] [y] [z] [dataTag]",
            "commands.teleport.usage" => "/teleport <entity> <x> <y> <z> [<y-rot> <x-rot>]",
            "commands.message.usage" => "/tell <player> <private message ...>",
            "commands.tellraw.usage" => "/tellraw <player> <raw json message>",
            "commands.testfor.usage" => "/testfor <player> [dataTag]",
            "commands.testforblock.usage" => "/testforblock <x> <y> <z> <block> [dataValue:-1:state:*] [dataTag]",
            "commands.compare.usage" => "/testforblocks <x1> <y1> <z1> <x2> <y2> <z2> <x> <y> <z> [mode]",
            "commands.time.usage" => "/time <set:add:query> <value>",
            "commands.title.usage" => "/title <player> title:subtitle:actionbar:clear:reset:times ...",
            "commands.title.usage.title" => "/title <player> title:subtitle:actionbar <raw json title>",
            "commands.title.usage.times" => "/title <player> times <fadeIn> <stay> <fadeOut>",
            "commands.downfall.usage" => "/toggledownfall",
            "commands.tp.usage" => "/tp [target player] <destination player> OR /tp [target player] <x> <y> <z> [<yaw> <pitch>]",
            "commands.trigger.usage" => "/trigger <objective> <add:set> <value>",
            "commands.weather.usage" => "/weather <clear:rain:thunder> [duration in seconds]",
            "commands.whitelist.usage" => "/whitelist <on:off:list:add:remove:reload>",
            "commands.whitelist.add.usage" => "/whitelist add <player>",
            "commands.whitelist.remove.usage" => "/whitelist remove <player>",
            "commands.worldborder.usage" => "/worldborder <set:center:damage:warning:get:add> ...",
            "commands.worldborder.add.usage" => "/worldborder add <sizeInBlocks> [timeInSeconds]",
            "commands.worldborder.set.usage" => "/worldborder set <sizeInBlocks> [timeInSeconds]",
            "commands.worldborder.center.usage" => "/worldborder center <x> <z>",
            "commands.worldborder.damage.usage" => "/worldborder damage <buffer:amount> ...",
            "commands.worldborder.warning.usage" => "/worldborder warning <time:distance> ...",
            "commands.xp.usage" => "/xp <amount> [player] OR /xp <amount>L [player]",

            "block.minecraft.dirt" => "Dirt",
            "item.minecraft.diamond" => "Diamond",


            _ => str,
        }
}
