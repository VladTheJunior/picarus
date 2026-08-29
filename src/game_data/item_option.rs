use std::{collections::BTreeSet, io::SeekFrom};

use crate::game_data::{
    AsyncBufReadExtReadString, DataFormat, TagType,
    effects::ItemMinMaxEffect,
    game_class::GameClass,
    item::{ArmorClassKind, ArmorTypes, ItemSubType, ReadableItem},
};
use anyhow::Result;

use gpui::SharedString;
use indexmap::IndexMap;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

#[derive(Default)]
pub struct ItemOption {
    pub level: u16,
    pub effect1: Vec<ItemMinMaxEffect>,
    pub effect2: Vec<ItemMinMaxEffect>,
    pub special_effect: Vec<ItemMinMaxEffect>,
    pub wr_effect1: Vec<ItemMinMaxEffect>,
    pub wr_effect2: Vec<ItemMinMaxEffect>,
    pub gd_effect1: Vec<ItemMinMaxEffect>,
    pub gd_effect2: Vec<ItemMinMaxEffect>,
    pub tf_effect1: Vec<ItemMinMaxEffect>,
    pub tf_effect2: Vec<ItemMinMaxEffect>,
    pub pr_effect1: Vec<ItemMinMaxEffect>,
    pub pr_effect2: Vec<ItemMinMaxEffect>,
    pub wz_effect1: Vec<ItemMinMaxEffect>,
    pub wz_effect2: Vec<ItemMinMaxEffect>,
    pub ac_effect1: Vec<ItemMinMaxEffect>,
    pub ac_effect2: Vec<ItemMinMaxEffect>,
    pub do_effect1: Vec<ItemMinMaxEffect>,
    pub do_effect2: Vec<ItemMinMaxEffect>,
}

impl ReadableItem for ItemOption {
    const FORMAT: DataFormat = DataFormat::String;
    type Key = u16;

    fn key(&self) -> Self::Key {
        self.level
    }

    type CollectionItem = Self;
    fn new_collection_item(item: Self) -> Self::CollectionItem {
        item
    }

    fn debug_mut(&mut self) -> &mut Vec<u8> {
        unimplemented!()
    }

    async fn read<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        mut self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        definitions: &IndexMap<SharedString, TagType>,
        global_offset: u64,
    ) -> Result<Self> {
        let tag_count = definitions.len();
        // Read all fields sequentially
        for tag_idx in 0..tag_count {
            let global_idx = item_idx * tag_count + tag_idx;
            let offset = offsets[global_idx] as u64;
            match Self::FORMAT {
                DataFormat::String => {
                    reader.seek(SeekFrom::Start(global_offset + offset)).await?;
                }
                DataFormat::WideString => {
                    reader.seek(SeekFrom::Start(global_offset + offset * 2)).await?;
                }
            };

            match tag_idx {
                0 => {
                    self.level = reader.read_f32_le().await? as u16;
                }
                1 => {
                    let effects = reader.read_string(Self::FORMAT).await?;

                    self.effect1 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                2 => {
                    let effects = reader.read_string(Self::FORMAT).await?;

                    self.effect2 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }

                3 => {
                    let effects = reader.read_string(Self::FORMAT).await?;
                    self.special_effect = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                4 => {
                    let effects = reader.read_string(Self::FORMAT).await?;
                    self.wr_effect1 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                5 => {
                    let effects = reader.read_string(Self::FORMAT).await?;
                    self.wr_effect2 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                6 => {
                    let effects = reader.read_string(Self::FORMAT).await?;
                    self.gd_effect1 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                7 => {
                    let effects = reader.read_string(Self::FORMAT).await?;
                    self.gd_effect2 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                8 => {
                    let effects = reader.read_string(Self::FORMAT).await?;
                    self.tf_effect1 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                9 => {
                    let effects = reader.read_string(Self::FORMAT).await?;
                    self.tf_effect2 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                10 => {
                    let effects = reader.read_string(Self::FORMAT).await?;
                    self.pr_effect1 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                11 => {
                    let effects = reader.read_string(Self::FORMAT).await?;
                    self.pr_effect2 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                12 => {
                    let effects = reader.read_string(Self::FORMAT).await?;
                    self.wz_effect1 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                13 => {
                    let effects = reader.read_string(Self::FORMAT).await?;
                    self.wz_effect2 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                14 => {
                    let effects = reader.read_string(Self::FORMAT).await?;
                    self.ac_effect1 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                15 => {
                    let effects = reader.read_string(Self::FORMAT).await?;
                    self.ac_effect2 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }

                16 => {
                    let effects = reader.read_string(Self::FORMAT).await?;
                    self.do_effect1 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                17 => {
                    let effects = reader.read_string(Self::FORMAT).await?;
                    self.do_effect2 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                _ => {}
            }
        }
        Ok(self)
    }
}

impl ItemOption {
    pub fn get_random_effects(&self, usable_class: &BTreeSet<GameClass>, item_sub_type: &str) -> Option<Vec<ItemMinMaxEffect>> {
        let item_sub_type = ItemSubType::try_from(item_sub_type).ok()?;
        let mut effects = vec![];
        let classes: Vec<&GameClass> = usable_class.iter().collect();
        let oe: Option<(&Vec<ItemMinMaxEffect>, &Vec<ItemMinMaxEffect>)> = match classes.as_slice() {
            [GameClass::Berserker] => Some((self.wr_effect1.as_ref(), self.wr_effect2.as_ref())),
            [GameClass::Guardian] => Some((self.gd_effect1.as_ref(), self.gd_effect2.as_ref())),
            [GameClass::Wizard, GameClass::Magician] | [GameClass::Magician, GameClass::Wizard] => {
                Some((self.wz_effect1.as_ref(), self.wz_effect2.as_ref()))
            }
            [GameClass::Trickster] => Some((self.do_effect1.as_ref(), self.do_effect2.as_ref())),
            [GameClass::Assassin] => Some((self.tf_effect1.as_ref(), self.tf_effect2.as_ref())),
            [GameClass::Priest] => Some((self.pr_effect1.as_ref(), self.pr_effect2.as_ref())),
            [GameClass::Ranger] => Some((self.ac_effect1.as_ref(), self.ac_effect2.as_ref())),
            _ => None,
        };

        match item_sub_type {
            // +
            ItemSubType::Necklage => {
                effects.push(self.effect2.get(0).cloned());
                effects.push(self.effect2.get(1).cloned());
                effects.push(self.effect2.get(9).cloned());
                effects.push(self.effect1.get(0).cloned());
                effects.push(self.effect1.get(4).cloned());
                effects.push(self.effect2.get(4).cloned());
                effects.push(self.effect2.get(10).cloned());
                effects.push(self.effect2.get(3).cloned());
                effects.push(self.effect2.get(2).cloned());
                effects.push(self.effect2.get(5).cloned());
            }
            // +
            ItemSubType::Ring => {
                effects.push(self.effect1.get(10).cloned());
                effects.push(self.effect1.get(1).cloned());
                effects.push(self.effect1.get(3).cloned());
                effects.push(self.effect1.get(7).cloned());
                effects.push(self.effect1.get(11).cloned());
                effects.push(self.effect2.get(6).cloned());
                effects.push(self.effect2.get(8).cloned());
                effects.push(self.effect1.get(9).cloned());
                effects.push(self.effect2.get(7).cloned());
                effects.push(self.effect1.get(8).cloned());
            }
            // +
            ItemSubType::Dagger => {
                let (oe1, oe2) = oe?;
                effects.push(oe1.get(0).cloned());
                effects.push(oe1.get(1).cloned());
                effects.push(oe1.get(2).cloned());
                effects.push(oe2.get(0).cloned());
                effects.push(oe2.get(1).cloned());
                effects.push(oe1.get(5).cloned());
            }
            // +
            ItemSubType::Sword | ItemSubType::Greatsword => {
                let (oe1, oe2) = oe?;
                effects.push(oe1.get(0).cloned());
                effects.push(oe1.get(2).cloned());
                effects.push(oe2.get(0).cloned());
                effects.push(oe2.get(1).cloned());
                effects.push(oe1.get(5).cloned());
            }
            // +
            ItemSubType::Lance => {
                effects.push(self.effect1.get(0).cloned());
                effects.push(self.effect1.get(1).cloned());
                effects.push(self.effect1.get(2).cloned());
                effects.push(self.effect1.get(3).cloned());
                effects.push(self.effect1.get(4).cloned());
                effects.push(self.effect2.get(0).cloned());
                effects.push(self.effect2.get(1).cloned());
                effects.push(self.effect1.get(5).cloned());
                effects.push(self.effect2.get(10).cloned());
                effects.push(self.effect1.get(6).cloned());
                effects.push(self.effect2.get(2).cloned());
                effects.push(self.effect2.get(3).cloned());
            }
            // +
            ItemSubType::Crossbow => {
                effects.push(self.effect1.get(0).cloned());
                effects.push(self.effect1.get(1).cloned());
                effects.push(self.effect1.get(2).cloned());
                effects.push(self.effect1.get(3).cloned());
                effects.push(self.effect1.get(4).cloned());
                effects.push(self.effect2.get(0).cloned());
                effects.push(self.effect2.get(1).cloned());
                effects.push(self.effect1.get(5).cloned());
                effects.push(self.effect1.get(11).cloned());
                effects.push(self.effect1.get(6).cloned());
                effects.push(self.effect2.get(2).cloned());
                effects.push(self.effect2.get(3).cloned());
            }
            // +
            ItemSubType::Scepter | ItemSubType::Bow | ItemSubType::Staff | ItemSubType::Wand => {
                let (oe1, oe2) = oe?;
                effects.push(oe1.get(2).cloned());
                effects.push(oe1.get(3).cloned());
                effects.push(oe1.get(4).cloned());
                effects.push(oe2.get(2).cloned());
                effects.push(oe2.get(3).cloned());
                effects.push(oe1.get(6).cloned());
            }
            // +
            ItemSubType::Shield => {
                let (oe1, oe2) = oe?;
                effects.push(oe1.get(0).cloned());
                effects.push(oe1.get(2).cloned());
                effects.push(oe1.get(9).cloned());
                effects.push(oe2.get(6).cloned());
                effects.push(oe2.get(9).cloned());
                effects.push(oe1.get(8).cloned());
            }
            // +
            ItemSubType::Vambrace => {
                let (oe1, oe2) = oe?;
                effects.push(oe1.get(2).cloned());
                effects.push(oe1.get(3).cloned());
                effects.push(oe1.get(4).cloned());
                effects.push(oe1.get(8).cloned());
                effects.push(oe2.get(5).cloned());
                effects.push(oe2.get(9).cloned());
                effects.push(oe1.get(10).cloned());
            }
            // +
            ItemSubType::TeddyBear | ItemSubType::Crest => {
                let (oe1, oe2) = oe?;
                effects.push(oe1.get(2).cloned());
                effects.push(oe1.get(3).cloned());
                effects.push(oe1.get(4).cloned());
                effects.push(oe1.get(9).cloned());
                effects.push(oe2.get(5).cloned());
                effects.push(oe2.get(8).cloned());
            }
            ItemSubType::Armor(armor_kind) => match armor_kind {
                ArmorClassKind::Physical(armor_types) => match armor_types {
                    ArmorTypes::Helmet => {
                        let (oe1, oe2) = oe?;
                        effects.push(oe1.get(0).cloned());
                        effects.push(oe1.get(1).cloned());
                        effects.push(oe1.get(2).cloned());
                        effects.push(oe1.get(3).cloned());
                        effects.push(oe1.get(4).cloned());
                        effects.push(oe1.get(7).cloned());
                        effects.push(oe2.get(9).cloned());
                        effects.push(oe1.get(9).cloned());
                        effects.push(oe1.get(8).cloned());
                    }
                    ArmorTypes::Pauldron => {
                        let (oe1, oe2) = oe?;
                        effects.push(oe1.get(0).cloned());
                        effects.push(oe1.get(1).cloned());
                        effects.push(oe1.get(2).cloned());
                        effects.push(oe1.get(3).cloned());
                        effects.push(oe1.get(4).cloned());
                        effects.push(oe1.get(7).cloned());
                        effects.push(oe2.get(10).cloned());
                        effects.push(oe1.get(11).cloned());
                        effects.push(oe1.get(8).cloned());
                    }
                    ArmorTypes::Armor => {
                        let (oe1, _) = oe?;
                        effects.push(oe1.get(0).cloned());
                        effects.push(oe1.get(1).cloned());
                        effects.push(oe1.get(2).cloned());
                        effects.push(oe1.get(3).cloned());
                        effects.push(oe1.get(4).cloned());
                        effects.push(oe1.get(7).cloned());
                        effects.push(oe1.get(9).cloned());
                        effects.push(oe1.get(10).cloned());
                        effects.push(oe1.get(8).cloned());
                    }
                    ArmorTypes::Gloves => {
                        let (oe1, oe2) = oe?;
                        effects.push(oe1.get(0).cloned());
                        effects.push(oe1.get(1).cloned());
                        effects.push(oe1.get(2).cloned());
                        effects.push(oe1.get(3).cloned());
                        effects.push(oe1.get(4).cloned());
                        effects.push(oe1.get(7).cloned());
                        effects.push(oe2.get(1).cloned());
                        effects.push(oe2.get(0).cloned());
                        effects.push(oe1.get(8).cloned());
                    }
                    ArmorTypes::Boots => {
                        let (oe1, _) = oe?;
                        effects.push(oe1.get(0).cloned());
                        effects.push(oe1.get(1).cloned());
                        effects.push(oe1.get(2).cloned());
                        effects.push(oe1.get(3).cloned());
                        effects.push(oe1.get(4).cloned());
                        effects.push(oe1.get(7).cloned());
                        effects.push(oe1.get(10).cloned());
                        effects.push(oe1.get(8).cloned());
                    }
                },
                ArmorClassKind::Magic(armor_types) => match armor_types {
                    ArmorTypes::Helmet => {
                        let (oe1, oe2) = oe?;
                        effects.push(oe1.get(0).cloned());
                        effects.push(oe1.get(1).cloned());
                        effects.push(oe1.get(2).cloned());
                        effects.push(oe1.get(3).cloned());
                        effects.push(oe1.get(4).cloned());
                        effects.push(oe1.get(7).cloned());
                        effects.push(oe2.get(9).cloned());
                        effects.push(oe1.get(8).cloned());
                        effects.push(oe1.get(10).cloned());
                    }
                    ArmorTypes::Pauldron => {
                        let (oe1, oe2) = oe?;
                        effects.push(oe1.get(0).cloned());
                        effects.push(oe1.get(1).cloned());
                        effects.push(oe1.get(2).cloned());
                        effects.push(oe1.get(3).cloned());
                        effects.push(oe1.get(4).cloned());
                        effects.push(oe1.get(7).cloned());
                        effects.push(oe2.get(10).cloned());
                        effects.push(oe1.get(11).cloned());
                        effects.push(oe1.get(10).cloned());
                    }
                    ArmorTypes::Armor => {
                        let (oe1, _) = oe?;
                        effects.push(oe1.get(0).cloned());
                        effects.push(oe1.get(1).cloned());
                        effects.push(oe1.get(2).cloned());
                        effects.push(oe1.get(3).cloned());
                        effects.push(oe1.get(4).cloned());
                        effects.push(oe1.get(7).cloned());
                        effects.push(oe1.get(8).cloned());
                        effects.push(oe1.get(9).cloned());
                        effects.push(oe1.get(10).cloned());
                    }
                    ArmorTypes::Gloves => {
                        let (oe1, oe2) = oe?;
                        effects.push(oe1.get(0).cloned());
                        effects.push(oe1.get(1).cloned());
                        effects.push(oe1.get(2).cloned());
                        effects.push(oe1.get(3).cloned());
                        effects.push(oe1.get(4).cloned());
                        effects.push(oe1.get(7).cloned());
                        effects.push(oe2.get(3).cloned());
                        effects.push(oe2.get(2).cloned());
                        effects.push(oe1.get(10).cloned());
                    }
                    ArmorTypes::Boots => {
                        let (oe1, _) = oe?;
                        effects.push(oe1.get(0).cloned());
                        effects.push(oe1.get(1).cloned());
                        effects.push(oe1.get(2).cloned());
                        effects.push(oe1.get(3).cloned());
                        effects.push(oe1.get(4).cloned());
                        effects.push(oe1.get(7).cloned());
                        effects.push(oe1.get(9).cloned());
                        effects.push(oe1.get(10).cloned());
                    }
                },
            },
        };

        let collection: Vec<_> = effects.into_iter().filter_map(|f| f).collect();
        (!collection.is_empty()).then_some(collection)
    }
}
