use std::{

    collections::{BTreeMap, BTreeSet, HashMap}, io::{BufRead, Seek, SeekFrom}, sync::Arc,
};

use crate::game_data::{
    DataFormat, ReadExt, TagType, items::{Item, ItemNode, ReadableItem}, locale::Locale, skill::Skill,
};
use anyhow::Result;
use gpui_kit::SharedString;
use indexmap::IndexMap;
use parking_lot::Mutex;


#[derive(Default, Clone)]
pub struct FellowCombination {
    pub febuffgroupid: SharedString,
    pub title: SharedString,
    pub entries: IndexMap<u8, FellowCombinationEntry>,
    pub rewardtime: f32,
    pub rewardbuff: SharedString,
    pub cooltime: f32,
    pub lock: f32,
    pub reqcount: f32,
    pub combincond: SharedString,
    pub skills: Vec<Skill>,
}

#[derive(Clone)]
pub struct FellowCombinationEntry {
    pub reqfellow: Option<ItemNode>,
    pub reqpet: Option<ItemNode>,
    pub fellow: SharedString,
    pub hidden: f32,
}

impl ReadableItem for FellowCombination {
    const FORMAT: DataFormat = DataFormat::String;
    type Key = SharedString;

    fn key(&self) -> Self::Key {
        unimplemented!()
    }

    type CollectionItem = Self;
    fn new_collection_item(item: Self) -> Self::CollectionItem {
        item
    }

    fn debug_mut(&mut self) -> &mut Vec<u8> {
        unimplemented!()
    }

    fn read<R: BufRead + Seek + ?Sized>(
        mut self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        definitions: &IndexMap<SharedString, TagType>,
        global_offset: u64,
    ) -> Result<Self> {
        let tag_count = definitions.len();
        let mut reqfellow: [Option<SharedString>; 5] = std::array::from_fn(|_| None);
        let mut reqpet: [Option<SharedString>; 5] = std::array::from_fn(|_| None);
        let mut fellow: [SharedString; 5] = std::array::from_fn(|_| SharedString::default());
        let mut hidden: [f32; 5] = [0.0; 5];
        for (tag_idx, (tag, _)) in definitions.iter().enumerate() {
            let global_idx = item_idx * tag_count + tag_idx;
            let offset = offsets[global_idx] as u64;
            match Self::FORMAT {
                DataFormat::String => {
                    reader.seek(SeekFrom::Start(global_offset + offset))?;
                }
                DataFormat::WideString => {
                    reader.seek(SeekFrom::Start(global_offset + offset * 2))?;
                }
            };
            match tag.as_str() {
                "febuffgroupid" => self.febuffgroupid = reader.read_string(Self::FORMAT)?,
                "타이틀" => self.title = reader.read_string(Self::FORMAT)?,

                "reqfellow01" => {
                    let id = SharedString::new(reader.read_string(Self::FORMAT)?.to_uppercase());
                    if id != "*" {
                        reqfellow[0] = Some(id)
                    }
                }
                "reqpet01" => {
                    let id = SharedString::new(reader.read_string(Self::FORMAT)?.to_uppercase());
                    if id != "*" {
                        reqpet[0] = Some(id)
                    }
                }
                "펠로우1" => fellow[0] = reader.read_string(Self::FORMAT)?,
                "hidden01" => hidden[0] = reader.read_f32_le()?,

                "reqfellow02" => {
                    let id = SharedString::new(reader.read_string(Self::FORMAT)?.to_uppercase());
                    if id != "*" {
                        reqfellow[1] = Some(id)
                    }
                }
                "reqpet02" => {
                    let id = SharedString::new(reader.read_string(Self::FORMAT)?.to_uppercase());
                    if id != "*" {
                        reqpet[1] = Some(id)
                    }
                }
                "펠로우2" => fellow[1] = reader.read_string(Self::FORMAT)?,
                "hidden02" => hidden[1] = reader.read_f32_le()?,

                "reqfellow03" => {
                    let id = SharedString::new(reader.read_string(Self::FORMAT)?.to_uppercase());
                    if id != "*" {
                        reqfellow[2] = Some(id)
                    }
                }
                "reqpet03" => {
                    let id = SharedString::new(reader.read_string(Self::FORMAT)?.to_uppercase());
                    if id != "*" {
                        reqpet[2] = Some(id)
                    }
                }
                "펠로우3" => fellow[2] = reader.read_string(Self::FORMAT)?,
                "hidden03" => hidden[2] = reader.read_f32_le()?,

                "reqfellow04" => {
                    let id = SharedString::new(reader.read_string(Self::FORMAT)?.to_uppercase());
                    if id != "*" {
                        reqfellow[3] = Some(id)
                    }
                }
                "reqpet04" => {
                    let id = SharedString::new(reader.read_string(Self::FORMAT)?.to_uppercase());
                    if id != "*" {
                        reqpet[3] = Some(id)
                    }
                }
                "펠로우4" => fellow[3] = reader.read_string(Self::FORMAT)?,
                "hidden04" => hidden[3] = reader.read_f32_le()?,

                "reqfellow05" => {
                    let id = SharedString::new(reader.read_string(Self::FORMAT)?.to_uppercase());
                    if id != "*" {
                        reqfellow[4] = Some(id)
                    }
                }
                "reqpet05" => {
                    let id = SharedString::new(reader.read_string(Self::FORMAT)?.to_uppercase());
                    if id != "*" {
                        reqpet[4] = Some(id)
                    }
                }
                "펠로우5" => fellow[4] = reader.read_string(Self::FORMAT)?,
                "hidden05" => hidden[4] = reader.read_f32_le()?,

                "rewardtime" => self.rewardtime = reader.read_f32_le()?,
                "rewardbuff" => self.rewardbuff = reader.read_string(Self::FORMAT)?,
                "cooltime" => self.cooltime = reader.read_f32_le()?,
                "lock" => self.lock = reader.read_f32_le()?,
                "reqcount" => self.reqcount = reader.read_f32_le()?,
                "combincond" => self.combincond = reader.read_string(Self::FORMAT)?,

                _ => {}
            }
        }
        for i in 0..5 {
            if reqfellow[i].is_some() || reqpet[i].is_some() {
                self.entries.insert(
                    i as u8,
                    FellowCombinationEntry {
                        reqfellow: reqfellow[i].clone().map(|f| ItemNode { id: f, item: None }),
                        reqpet: reqpet[i].clone().map(|f| ItemNode { id: f, item: None }),
                        fellow: fellow[i].clone(),
                        hidden: hidden[i],
                    },
                );
            }
        }
        Ok(self)
    }
}

impl FellowCombination {
    pub fn set_fellows(&mut self, items: &IndexMap<SharedString, Arc<Mutex<Item>>>, unknown_ids: &mut BTreeMap<SharedString, u32>) {
        for (_, entry) in self.entries.iter_mut() {
            if let Some(node) = entry.reqfellow.as_mut() {
                node.item = items.get(&node.id).map(|f| Arc::downgrade(f));
                if node.item.is_none() {
                    unknown_ids.entry(node.id.clone()).and_modify(|count| *count += 1).or_insert(1);
                }
            }

            if let Some(node) = entry.reqpet.as_mut() {
                node.item = items.get(&node.id).map(|f| Arc::downgrade(f));
                if node.item.is_none() {
                    unknown_ids.entry(node.id.clone()).and_modify(|count| *count += 1).or_insert(1);
                }
            }
        }
    }

    pub fn set_skills(
        &mut self,
        skills: &HashMap<SharedString, Skill>,
        skill_locales: &HashMap<SharedString, Locale>,
        unknown_skills: &mut BTreeSet<SharedString>,
    ) -> Result<()> {
        for name in self.rewardbuff.split(",").filter(|s| !s.is_empty()) {
            let clear_name = SharedString::new(name.split_once(".").map(|(a, _)| a).unwrap_or_else(|| name).trim().to_uppercase());
            if let Some(skill) = skills.get(&clear_name) {
                let mut skill = skill.clone();
                if let Some(level) = name.split_once(".").and_then(|(_, a)| a.trim().parse::<u8>().ok()) {
                    skill.max_item_level = level;
                }
                skill.description_locale = skill_locales
                    .get(&SharedString::new(name.to_uppercase().replace(".", "_DESCRIPTION_")))
                    .cloned();
                self.skills.push(skill);
            } else {
                unknown_skills.insert(SharedString::new(clear_name));
            }
        }

        Ok(())
    }
}
