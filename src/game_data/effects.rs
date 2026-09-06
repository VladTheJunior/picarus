use fluent::FluentValue;
use gpui_kit::SharedString;

use crate::language::t_v;

pub enum EffectKind {
    Common { id: SharedString, effect: ItemEffect },
    MinMaxNoStep { id: SharedString, effect: ItemMinMaxNoStepEffect },
    MinMaxStep { id: SharedString, effect: ItemMinMaxStepEffect },
}

#[derive(Default, Clone)]
pub struct ItemEffect {
    pub effect: SharedString,
    pub intermediate_effect: Option<SharedString>,
    pub parsed: Option<(SharedString, f32)>,
}

#[derive(Default, Clone)]
pub struct ItemMinMaxStepEffect {
    pub effect: SharedString,
    pub intermediate_effect: Option<SharedString>,
    pub parsed: Option<(SharedString, f32, f32, f32)>,
}
#[derive(Default, Clone)]
pub struct ItemMinMaxNoStepEffect {
    pub effect: SharedString,
    pub intermediate_effect: Option<SharedString>,
    pub parsed: Option<(SharedString, f32, f32)>,
}

#[derive(Default, Clone)]
pub struct ItemMinMaxEffect {
    pub effect: SharedString,
    pub parsed: Option<(SharedString, f32, f32)>,
}

impl ItemMinMaxStepEffect {
    pub fn new(effect: &str) -> Self {
        let mut e = Self::default();
        e.effect = SharedString::new(effect);
        e.parse_effect();
        e
    }

    pub fn get_locale(&self, maximized: bool, tempering_effect: f32) -> SharedString {
        self.parsed
            .as_ref()
            .map(|(key, min, max, step)| {
                let (min, max) = if maximized {
                    (
                        (min + step) * (1.0 + tempering_effect / 100.0),
                        (max + step) * (1.0 + tempering_effect / 100.0),
                    )
                } else {
                    (min * (1.0 + tempering_effect / 100.0), max * (1.0 + tempering_effect / 100.0))
                };

                if key.ends_with("-minus-percent") {
                    t_v(key, vec![("value", format!("{:.2}% ~ -{:.2}", min, max))])
                } else if key.ends_with("-percent") {
                    t_v(key, vec![("value", format!("{:.2}% ~ {:.2}", min, max))])
                } else {
                    t_v(key, vec![("value", format!("{:.0} ~ {:.0}", min, max))])
                }
            })
            .and_then(|s| if s.is_empty() { None } else { Some(s) })
            .unwrap_or_else(|| self.effect.clone())
    }
    fn parse_key_min_max_step(input: &str) -> Option<(&str, f32, f32, f32)> {
        let parts: Vec<&str> = input.split(',').collect();
        if parts.len() != 4 {
            return None;
        }

        let key = parts[0];
        let min = parts[1].parse::<f32>().ok()?;
        let max = parts[2].parse::<f32>().ok()?;
        let step = parts[3].parse::<f32>().ok()?;

        Some((key, min, max, step))
    }

    fn parse_effect(&mut self) {
        if let Some((effect_key, min, max, step)) = Self::parse_key_min_max_step(&self.effect) {
            self.intermediate_effect = Some(SharedString::new(effect_key));
            if let Some(effect_key) = ItemEffect::matching(effect_key) {
                self.parsed = Some((SharedString::new(effect_key), min, max, step));
            }
        }
    }
}

impl ItemMinMaxNoStepEffect {
    pub fn new(effect: &str) -> Self {
        let mut e = Self::default();
        e.effect = SharedString::new(effect);
        e.parse_effect();
        e
    }

    pub fn get_locale(&self, tempering_effect: f32) -> SharedString {
        self.parsed
            .as_ref()
            .map(|(key, min, max)| {
                let min = (min) * (1.0 + tempering_effect / 100.0);
                let max = (max) * (1.0 + tempering_effect / 100.0);

                if key.ends_with("-minus-percent") {
                    t_v(key, vec![("value", format!("{:.2}% ~ -{:.2}", min, max))])
                } else if key.ends_with("-percent") {
                    t_v(key, vec![("value", format!("{:.2}% ~ {:.2}", min, max))])
                } else {
                    t_v(key, vec![("value", format!("{:.0} ~ {:.0}", min, max))])
                }
            })
            .and_then(|s| if s.is_empty() { None } else { Some(s) })
            .unwrap_or_else(|| self.effect.clone())
    }
    fn parse_key_min_max(input: &str) -> Option<(&str, f32, f32)> {
        let parts: Vec<&str> = input.split(',').collect();
        if parts.len() != 3 {
            return None;
        }

        let key = parts[0];
        let min = parts[1].parse::<f32>().ok()?;
        let max = parts[2].parse::<f32>().ok()?;

        Some((key, min, max))
    }

    fn parse_effect(&mut self) {
        if let Some((effect_key, min, max)) = Self::parse_key_min_max(&self.effect) {
            self.intermediate_effect = Some(SharedString::new(effect_key));
            if let Some(effect_key) = ItemEffect::matching(effect_key) {
                self.parsed = Some((SharedString::new(effect_key), min, max));
            }
        }
    }
}

impl ItemMinMaxEffect {
    pub fn new(effect: &str) -> Self {
        let mut e = Self::default();
        e.effect = SharedString::new(effect);
        e.parse_effect();
        e
    }

    pub fn get_locale(&self) -> SharedString {
        self.parsed
            .as_ref()
            .map(|(key, min, max)| {
                if key.ends_with("-minus-percent") {
                    t_v(key, vec![("value", format!("{:.2}% ~ -{:.2}", min, max))])
                } else if key.ends_with("-percent") {
                    t_v(key, vec![("value", format!("{:.2}% ~ {:.2}", min, max))])
                } else {
                    t_v(key, vec![("value", format!("{:.0} ~ {:.0}", min, max))])
                }
            })
            .and_then(|s| if s.is_empty() { None } else { Some(s) })
            .unwrap_or_else(|| self.effect.clone())
    }
    fn parse_key_min_max(input: &str) -> Option<(&str, f32, f32)> {
        let parts: Vec<&str> = input.split('_').collect();
        if parts.len() != 3 {
            return None;
        }

        let key = parts[0];
        let min = parts[1].parse::<f32>().ok()?;
        let max = parts[2].parse::<f32>().ok()?;

        Some((key, min, max))
    }

    fn parse_effect(&mut self) {
        if let Some((effect_key, min, max)) = Self::parse_key_min_max(&self.effect) {
            if let Some(effect_key) = ItemEffect::matching(effect_key) {
                self.parsed = Some((SharedString::new(effect_key), min, max));
            }
        }
    }
}

impl ItemEffect {
    pub fn matching(key: &str) -> Option<&str> {
        match key.to_ascii_lowercase().as_str() {
            "최대ep%" => Some("item-effect-max-ep-percent"),
            "생명력흡수성공확률+" | "생명력흡수성공확률%" => Some("item-effect-health-absorption-chance-percent"),
            "생명력흡수량+" => Some("item-effect-health-absorption-amount-percent"),
            "데미지감소%" => Some("item-effect-damage-reduction-percent"),
            "석궁피격데미지%" => Some("item-effect-crossbow-damage-percent"),
            "창피격데미지%" => Some("item-effect-lance-damage-percent"),
            "창피격데미지%-" => Some("item-effect-lance-damage-minus-percent"),
            "배후공격극대화확률+" => Some("item-effect-backstab-damage"),
            "회피력+" => Some("item-effect-evasion-power"),
            "회피율%" | "회피율+" => Some("item-effect-evasion-percent"), // хз, уклонение, проверить на Capital Guard Veiled Gloves
            "최대mp+" => Some("item-effect-mana"),
            "최대hp+" => Some("item-effect-max-hp"),
            "최대hp%" => Some("item-effect-max-hp-percent"),
            "무기물리방어력%" => Some("item-effect-physical-defense-percent"),
            "쿨타임%" => Some("item-effect-cooldown-percent"),
            "pk방어력%" => Some("item-effect-pvp-defense-percent"),
            "모든공격력+" => Some("item-effect-attack"),
            "모든공격력%" => Some("item-effect-attack-percent"),
            "allstatderest+" => Some("item-effect-stat-limit-break"),
            "allstatderest%" => Some("item-effect-stat-limit-break-percent"),
            "allstat+" => Some("item-effect-allstats"),
            "allstat%" => Some("item-effect-allstats-percent"),
            "모든극대화확률+" => Some("item-effect-crit-damage-chance-percent"),
            "pk육체계저항율+" => Some("item-effect-pvp-resist-percent"),
            "이동속도%" => Some("item-effect-speed-percent"),
            "탈것속도%" => Some("item-effect-mount-speed-percent"),
            "치명타피해감소+" => Some("item-effect-crit-defense"),
            "마법방어력%" => Some("item-effect-magic-defense-percent"),
            "intderest+" => Some("item-effect-intelligence-break-limit"),
            "intderest%" => Some("item-effect-intelligence-break-limit-percent"),
            "vtlderest+" => Some("item-effect-vitality-break-limit"),
            "vtlderest%" => Some("item-effect-vitality-break-limit-percent"),
            "strderest+" => Some("item-effect-strength-break-limit"),
            "strderest%" => Some("item-effect-strength-break-limit-percent"),
            "dexderest+" => Some("item-effect-dexterity-break-limit"),
            "dexderest%" => Some("item-effect-dexterity-break-limit-percent"),
            "mtlderest+" => Some("item-effect-mentality-break-limit"),
            "mtlderest%" => Some("item-effect-mentality-break-limit-percent"),
            "int%" => Some("item-effect-intelligence-percent"),
            "str%" => Some("item-effect-strength-percent"),
            "vtl%" => Some("item-effect-vitality-percent"),
            "mtl%" => Some("item-effect-mentality-percent"),
            "dex%" => Some("item-effect-dexterity-percent"),
            "vtl+" => Some("item-effect-vitality"),
            "mtl+" => Some("item-effect-mentality"),
            "int+" => Some("item-effect-intelligence"),
            "str+" => Some("item-effect-strength"),
            "dex+" => Some("item-effect-dexterity"),
            "bdy%" => Some("item-effect-physical-resistance-percent"),
            "기절상태" => Some("item-effect-stun"),
            // "신체독중독상태" => Some("item-effect-stun"),
            "pk공격력%" => Some("item-effect-pvp-attack-percent"),
            "출혈관통률" => Some("item-effect-bleed-chance-percent"),
            "모든방어력%" => Some("item-effect-defense-percent"),
            "모든방어력+" => Some("item-effect-defense"),
            "무기물리방어력+" => Some("item-effect-physical-defense"),
            "무기물리공격력+" => Some("item-effect-physical-attack"),
            "마법방어력+" => Some("item-effect-magic-defense"),
            "캐스팅속도%" => Some("item-effect-cast-time-percent"),
            "마법물리공격력+" => Some("item-effect-magic-attack"),
            /* idk about 2 */
            "출혈방어율" | "출혈방어율%" => Some("item-effect-bleed-defense-percent"),
            "모든극대력+" => Some("item-effect-critical-damage"),
            "마법극대력+" => Some("item-effect-magic-critical-damage"),
            "마법극대화데미지+" => Some("item-effect-magic-critical-damage-percent"),
            /* idk about 2, this one is uniq [Lazards Priest set effect] */
            "마법극대화확률+" | "마법극대화확률%" => Some("item-effect-magic-critical-damage-chance-percent"),
            "무기극대화확률+" => Some("item-effect-physical-critical-damage-chance-percent"),
            "치명타피해관통율%" => Some("item-effect-critical-damage-penetration-percent"),
            "무기극대력+" => Some("item-effect-physical-critical-damage"),
            "무기극대화데미지+" | "무기극대력%" => Some("item-effect-physical-critical-damage-percent"),
            "몬스터드랍율%" | "드랍율+" => Some("item-effect-drop-chance-percent"),
            "마법물리공격력%" => Some("item-effect-magic-attack-percent"),
            "무기물리공격력%" => Some("item-effect-physical-attack-percent"),
            "길들이기확률%" => Some("item-effect-taming-chance-percent"),
            "리버스강화확률%" => Some("item-effect-reverse-tempering-chance-percent"),
            "강화성공확률%" => Some("item-effect-tempering-chance-percent"),
            "제작성공확률%" => Some("item-effect-crafting-chance-percent"),
            "제작대성공확률%" => Some("item-effect-great-craft-chance-percent"),
            "판매대행등록비감소%" => Some("item-effect-auction-fee-percent"),
            "판매대행판매수수료감소%" => Some("item-effect-auction-sales-fee-percent"),
            "펠로우경험치%" | "접속중펠로우위탁경험치%" => Some("item-effect-mount-exp-percent"),
            "도트데미지감소+" => Some("item-effect-bleed-damage-reduction"), //idk
            "도트데미지감소%" => Some("item-effect-bleed-damage-reduction-percent"), //idk
            "길들이기포인트감소%" => Some("item-effect-taming-points-percent"), // проверить потом на бафе зелек
            "고도+" => Some("item-effect-mount-altitude"),
            "드랍money변화율*" => Some("item-effect-money-drop-increase-percent"),
            "money추가획득율%" => Some("item-effect-money-drop-increase"),
            "공격자의치명타피해plus효과감소%" => Some("item-effect-critical-defense-percent"),
            "최대mp%" => Some("item-effect-mana-percent"),
            "플레이어경험치%" => Some("item-effect-obtained-character-exp-percent"),

            "배후공격데미지%" => Some("item-effect-backstab-rate-percent"),
            "hp힐량%" => Some("item-effect-health-regen-percent"),
            "어그로%" => Some("item-effect-threat-percent"),
            "hp회복력%" => Some("item-effect-base-health-regen-percent"),
            "hp회복력+" => Some("item-effect-base-health-regen"),
            "mp회복력%" => Some("item-effect-base-mana-regen-percent"),
            "mp회복력+" => Some("item-effect-base-mana-regen"),
            "마법물리방어력+" => Some("item-effect-magic-and-physical-defense"),
            "낚시시간감소" => Some("item-effect-fishing-time-sec"),
            "펫포획확률%" => Some("item-effect-capturing-chance-percent"),
            "월척확률증가%" => Some("item-effect-fishing-very-rare-drop-percent"),
            "모든낚시확률증가%" => Some("item-effect-fishing-drop-percent"),
            "준척확률증가%" => Some("item-effect-fishing-rare-drop-percent"),
            "길드포인트%" => Some("item-effect-guild-points-percent"),
            _ => {
                return None;
            }
        }
    }

    pub fn new(effect: SharedString) -> Self {
        let mut e = Self::default();
        e.effect = effect;
        e.parse_effect();
        e
    }

    pub fn from_effect_and_value(effect: &str, value: Option<&SharedString>) -> Self {
        let mut e = Self::default();
        e.effect = SharedString::new(effect);
        e.parse_effect_with_value(effect, value);
        e
    }

    fn parse_key_value(s: &str) -> Option<(&str, f32)> {
        let s = s.trim_start_matches("(").trim_end_matches(")");

        let mut parts = s.splitn(2, ',');
        let key = parts.next()?.trim();
        let value_str = parts.next()?.trim();
        let value = value_str.trim_end_matches("%").parse::<f32>().ok()?;

        Some((key, value))
    }

    pub fn get_locale(&self) -> SharedString {
        self.parsed
            .as_ref()
            .map(|(key, value)| {
                if key.ends_with("-minus-percent") {
                    t_v(key, vec![("value", format!("{:.2}", value))])
                } else if key.ends_with("-percent") {
                    t_v(key, vec![("value", format!("{:+.2}", value))])
                } else {
                    t_v(key, vec![("value", format!("{:+.0}", value))])
                }
            })
            .and_then(|s| if s.is_empty() { None } else { Some(s) })
            .unwrap_or_else(|| self.effect.clone())
    }

    pub fn get_locale_with_duration(&self, duration: i32) -> SharedString {
        if duration > 0 {
            self.parsed
                .as_ref()
                .map(|(key, value)| {
                    if key.ends_with("-minus-percent") {
                        t_v(key, vec![("value", format!("{:.2}", value))])
                    } else if key.ends_with("-percent") {
                        t_v(key, vec![("value", format!("{:+.2}", value))])
                    } else {
                        t_v(key, vec![("value", format!("{:+.0}", value))])
                    }
                })
                .and_then(|s| {
                    if s.is_empty() {
                        None
                    } else {
                        Some(t_v(
                            "item-skill-effect-with-duration",
                            vec![
                                ("effect", FluentValue::from(s.as_str())),
                                ("duration", FluentValue::Number((duration / 1000).into())),
                            ],
                        ))
                    }
                })
                .unwrap_or_else(|| self.effect.clone())
        } else {
            self.get_locale()
        }
    }

    fn parse_effect(&mut self) {
        if let Some((effect_key, value)) = Self::parse_key_value(&self.effect) {
            self.intermediate_effect = Some(SharedString::new(effect_key));
            if let Some(effect_key) = Self::matching(effect_key) {
                self.parsed = Some((SharedString::new(effect_key), value));
            }
        }
    }

    fn parse_effect_with_value(&mut self, effect_key: &str, value_str: Option<&SharedString>) {
        self.intermediate_effect = Some(SharedString::new(effect_key));
        if let Some(value) = value_str.and_then(|f| f.trim_end_matches("%").parse::<f32>().ok()) {
            if let Some(effect_key) = Self::matching(effect_key) {
                self.parsed = Some((SharedString::new(effect_key), value));
            }
        }
    }
}
