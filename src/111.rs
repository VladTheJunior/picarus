


#[derive(Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize)]
pub enum _ArmorKind {
    HeavyArmor(ArmorTypes),
    LightArmorMagic(ArmorTypes),
    LightArmorPhysical(ArmorTypes),
    RobeArmor(ArmorTypes),
}





impl DataType {
    pub fn validate_grades(&self) {
        if self.get_grade().is_none() {
            let id = self.get_id();
            let name = self.get_locale_name();
            warn!(?id, ?name, "Can not detect grade");
        }
    }
    pub fn validate_effects(&self) {
        let id = self.get_id();
        let name = self.get_locale_name();
        let mut effects = Vec::new();
        match self {
            DataType::Weapon(weapon) => {
                [
                    &weapon.equip_effect_1,
                    &weapon.equip_effect_2,
                    &weapon.equip_effect_3,
                    &weapon.equip_effect_4,
                ]
                .iter()
                .filter_map(|opt| opt.as_ref())
                .for_each(|effect| effects.push(effect));
                if let Some(set) = weapon.item_set.as_ref() {
                    for e in &set.effects {
                        effects.extend(e.seteffect_effects.iter());
                    }
                }
            }
            DataType::Armor(armor) => {
                [&armor.equip_effect_1, &armor.equip_effect_2, &armor.equip_effect_3, &armor.equip_effect_4]
                    .iter()
                    .filter_map(|opt| opt.as_ref())
                    .for_each(|effect| effects.push(effect));
                if let Some(set) = armor.item_set.as_ref() {
                    for e in &set.effects {
                        effects.extend(e.seteffect_effects.iter());
                    }
                }
            }
            DataType::Accessory(accessory) => {
                [
                    &accessory.equip_effect_1,
                    &accessory.equip_effect_2,
                    &accessory.equip_effect_3,
                    &accessory.equip_effect_4,
                ]
                .iter()
                .filter_map(|opt| opt.as_ref())
                .for_each(|effect| effects.push(effect));
                if let Some(set) = accessory.item_set.as_ref() {
                    for e in &set.effects {
                        effects.extend(e.seteffect_effects.iter());
                    }
                }
            }
            DataType::SecondaryWeapon(secondary_weapon) => {
                [
                    &secondary_weapon.equip_effect_1,
                    &secondary_weapon.equip_effect_2,
                    &secondary_weapon.equip_effect_3,
                    &secondary_weapon.equip_effect_4,
                ]
                .iter()
                .filter_map(|opt| opt.as_ref())
                .for_each(|effect| effects.push(effect));
                if let Some(set) = secondary_weapon.item_set.as_ref() {
                    for e in &set.effects {
                        effects.extend(e.seteffect_effects.iter());
                    }
                }
            }
            DataType::Style(style) => {
                [&style.equip_effect_1, &style.equip_effect_2, &style.equip_effect_3, &style.equip_effect_4]
                    .iter()
                    .filter_map(|opt| opt.as_ref())
                    .for_each(|effect| effects.push(effect));
                if let Some(set) = style.item_set.as_ref() {
                    for e in &set.effects {
                        effects.extend(e.seteffect_effects.iter());
                    }
                }
            }
            DataType::Material(_) => {}
            DataType::Recipe(_) => {}
            DataType::Consume(_) => {}
            DataType::SkillBook(_) => {}
            DataType::Exchange(_) => {}
            DataType::RandomBox(_) => {}
            DataType::Package(_) => {}
            DataType::Bag(_) => {}
            DataType::SealedFellow(sealed_fellow) => {
                [
                    &sealed_fellow.sealed_fellow_effect_1,
                    &sealed_fellow.sealed_fellow_effect_2,
                    &sealed_fellow.sealed_fellow_effect_3,
                ]
                .iter()
                .filter_map(|opt| opt.as_ref())
                .filter(|f| f.parsed.is_none())
                .for_each(|e| warn!(?id, ?name, ?e.effect, "Failed to detect effect"));

                if let Some(e) = sealed_fellow.max_enhancement_sealed_fellow_effect.as_ref() {
                    if e.parsed.is_none() {
                        warn!(?id, ?name, ?e.effect, "Failed to detect effect");
                    }
                }
            }
            DataType::FellowEquip(fellow_equip) => {
                [
                    &fellow_equip.equip_effect_1,
                    &fellow_equip.equip_effect_2,
                    &fellow_equip.equip_effect_3,
                    &fellow_equip.equip_effect_4,
                ]
                .iter()
                .filter_map(|opt| opt.as_ref())
                .for_each(|effect| effects.push(effect));
                if let Some(set) = fellow_equip.item_set.as_ref() {
                    for e in &set.effects {
                        effects.extend(e.seteffect_effects.iter());
                    }
                }
            }
            DataType::Boost(boost) => {
                [&boost.equip_effect_1, &boost.equip_effect_2, &boost.equip_effect_3, &boost.equip_effect_4]
                    .iter()
                    .filter_map(|opt| opt.as_ref())
                    .for_each(|effect| effects.push(effect));
            }
            DataType::Gem(gem) => {
                [&gem.equip_effect_1, &gem.equip_effect_2, &gem.equip_effect_3, &gem.equip_effect_4]
                    .iter()
                    .filter_map(|opt| opt.as_ref())
                    .for_each(|effect| effects.push(effect));
            }
        };

        for e in effects.iter().filter(|f| f.parsed.is_none()) {
            warn!(?id, ?name, ?e.effect, "Failed to detect effect");
        }
    }

   

    



pub struct GameData {
    pub items: IndexMap<SharedString, Rc<DataType>>,
    pub effects_by_grade: HashMap<Grade, HashMap<u16, ItemOption>>,
    pub tempering_by_types: HashMap<SharedString, HashMap<u16, Tempering>>,
    pub quality_by_types: HashMap<SharedString, HashMap<u16, ItemQuality>>,
    pub products_by_recipe_id: HashMap<SharedString, Rc<RefCell<Product>>>,
    pub products_by_result_id: HashMap<SharedString, Rc<RefCell<Product>>>,
    pub random_box_groups: HashMap<SharedString, Rc<RefCell<RandomBoxGroup>>>,
}

impl GameData {



    pub async fn load(game_path: &str, on_load: &Entity<GameDataLoadingStatus>, cx: &mut AsyncWindowContext) -> Result<Self> {
        let gamedatas = File::open(Path::new(game_path).join(r"Game\gamedatas.npk"))?;
        let gamelibs = File::open(Path::new(game_path).join(r"Game\gamelibs.npk"))?;
        let mut gamedatas_zip = ZipArchive::new(gamedatas)?;
        let mut gamelibs_zip = ZipArchive::new(gamelibs)?;
        let mut data = Self::new();

     

        Ok(data)
    }
   

    async fn read_styles<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Style;
            cx.notify();
        });
        let locales = self.read_style_locales(gamedatas_zip).await?;
        let skill_locales = self.read_skill_locales(gamedatas_zip).await?;
        let res = self.read_style_itemres(gamedatas_zip).await?;
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemdata_style.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items(
            &data,
            DataFormat::String,
            DataType::Style,
            &locales,
            &skill_locales,
            &res,
            &item_set,
            gamelibs_zip,
        )
        .await
    }

    async fn read_packages<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Package;
            cx.notify();
        });
        let locales = self.read_package_locales(gamedatas_zip).await?;

        let res = self.read_package_itemres(gamedatas_zip).await?;
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemdata_package.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items(
            &data,
            DataFormat::String,
            DataType::Package,
            &locales,
            &HashMap::new(),
            &res,
            &item_set,
            gamelibs_zip,
        )
        .await
    }


    

    async fn read_random_boxes<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::RandomBox;
            cx.notify();
        });
        let locales = self.read_random_box_locales(gamedatas_zip).await?;

        let res = self.read_random_box_itemres(gamedatas_zip).await?;
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemdata_randombox.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items(
            &data,
            DataFormat::String,
            DataType::RandomBox,
            &locales,
            &HashMap::new(),
            &res,
            &item_set,
            gamelibs_zip,
        )
        .await
    }

    async fn read_random_box_groups<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::RandomBoxGroup;
            cx.notify();
        });

        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\randomboxtable_randomboxgroup.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_random_box_group(&data, DataFormat::String).await
    }

    async fn read_random_box_probabilities<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<HashMap<SharedString, RandomBoxProbability>> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::RandomBoxProbability;
            cx.notify();
        });

        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\randomboxtable_randomboxprobability.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_random_box_probability(&data, DataFormat::String).await
    }

        async fn read_bags<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Bag;
            cx.notify();
        });
        let locales = self.read_bag_locales(gamedatas_zip).await?;

        let res = self.read_bag_itemres(gamedatas_zip).await?;
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemdata_bag.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items(
            &data,
            DataFormat::String,
            DataType::Bag,
            &locales,
            &HashMap::new(),
            &res,
            &item_set,
            gamelibs_zip,
        )
        .await
    }

    async fn read_item_boost<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::RandomBoxGroup;
            cx.notify();
        });
        let locales = self.read_boost_locales(gamedatas_zip).await?;

        let res = self.read_boost_itemres(gamedatas_zip).await?;
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemdata_boost.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items(
            &data,
            DataFormat::String,
            DataType::Boost,
            &locales,
            &HashMap::new(),
            &res,
            &item_set,
            gamelibs_zip,
        )
        .await
    }

    async fn read_product_materials<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::ProductMaterial;
            cx.notify();
        });
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\productdata_productmaterial.bin")?;

        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_product_material(&data, DataFormat::String).await?;
        Ok(())
    }

    

   

    async fn read_itemset<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<Vec<ItemSet>> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::ItemSet;
            cx.notify();
        });
        let locales = self.read_itemset_locales(gamedatas_zip).await?;
        let skill_locales = self.read_skill_locales(gamedatas_zip).await?;
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemset_setcharacter.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_itemset(&data, DataFormat::String, &locales, &skill_locales).await
    }

    async fn read_itemset_fellow<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<Vec<ItemSet>> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::ItemSet;
            cx.notify();
        });
        let locales = self.read_itemset_locales(gamedatas_zip).await?;
        let skill_locales = self.read_skill_locales(gamedatas_zip).await?;
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemset_setfellow.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_itemset(&data, DataFormat::String, &locales, &skill_locales).await
    }

    async fn read_material<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Material;
            cx.notify();
        });

        let locales = self.read_material_locales(gamedatas_zip).await?;
        let res = self.read_material_itemres(gamedatas_zip).await?;
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemdata_material.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items(
            &data,
            DataFormat::String,
            DataType::Material,
            &locales,
            &HashMap::new(),
            &res,
            &item_set,
            gamelibs_zip,
        )
        .await
    }

    async fn read_accessory<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Accessory;
            cx.notify();
        });

        let locales = self.read_accessory_locales(gamedatas_zip).await?;
        let skill_locales = self.read_skill_locales(gamedatas_zip).await?;
        let res = self.read_accessory_itemres(gamedatas_zip).await?;
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemdata_accessory.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items(
            &data,
            DataFormat::String,
            DataType::Accessory,
            &locales,
            &skill_locales,
            &res,
            &item_set,
            gamelibs_zip,
        )
        .await
    }

    async fn read_consumes<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Consume;
            cx.notify();
        });
        let locales = self.read_consume_locales(gamedatas_zip).await?;

        let res = self.read_consume_itemres(gamedatas_zip).await?;
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemdata_consume.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items(
            &data,
            DataFormat::String,
            DataType::Consume,
            &locales,
            &HashMap::new(),
            &res,
            &item_set,
            gamelibs_zip,
        )
        .await
    }

    async fn read_recipes<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Recipe;
            cx.notify();
        });
        let locales = self.read_recipe_locales(gamedatas_zip).await?;
        let res = self.read_recipe_itemres(gamedatas_zip).await?;
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemdata_recipe.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items(
            &data,
            DataFormat::String,
            DataType::Recipe,
            &locales,
            &HashMap::new(),
            &res,
            &item_set,
            gamelibs_zip,
        )
        .await
    }

    async fn read_armors<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Armor;
            cx.notify();
        });
        let locales = self.read_armor_locales(gamedatas_zip).await?;
        let skill_locales = self.read_skill_locales(gamedatas_zip).await?;
        let res = self.read_armor_itemres(gamedatas_zip).await?;
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemdata_armor.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items(
            &data,
            DataFormat::String,
            DataType::Armor,
            &locales,
            &skill_locales,
            &res,
            &item_set,
            gamelibs_zip,
        )
        .await
    }

    async fn read_fellow_equips<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::FellowEquip;
            cx.notify();
        });
        let locales = self.read_fellow_equip_locales(gamedatas_zip).await?;
        let skill_locales = self.read_skill_locales(gamedatas_zip).await?;
        let res = self.read_fellow_equip_itemres(gamedatas_zip).await?;
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemdata_fellowequip.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items(
            &data,
            DataFormat::String,
            DataType::FellowEquip,
            &locales,
            &skill_locales,
            &res,
            &item_set,
            gamelibs_zip,
        )
        .await
    }

    async fn read_secondary_weapons<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::SecondaryWeapon;
            cx.notify();
        });
        let locales = self.read_secondary_weapon_locales(gamedatas_zip).await?;
        let skill_locales = self.read_skill_locales(gamedatas_zip).await?;
        let res = self.read_secondary_weapon_itemres(gamedatas_zip).await?;
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemdata_sub.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items(
            &data,
            DataFormat::String,
            DataType::SecondaryWeapon,
            &locales,
            &skill_locales,
            &res,
            &item_set,
            gamelibs_zip,
        )
        .await
    }

    async fn read_weapons<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Weapon;
            cx.notify();
        });
        let locales = self.read_weapon_locales(gamedatas_zip).await?;
        let skill_locales = self.read_skill_locales(gamedatas_zip).await?;
        let res = self.read_weapon_itemres(gamedatas_zip).await?;
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemdata_weapon.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items(
            &data,
            DataFormat::String,
            DataType::Weapon,
            &locales,
            &skill_locales,
            &res,
            &item_set,
            gamelibs_zip,
        )
        .await
    }

    async fn read_exchange<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Exchange;
            cx.notify();
        });
        let locales = self.read_exchange_locales(gamedatas_zip).await?;
        let res = self.read_exchange_itemres(gamedatas_zip).await?;
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemdata_exchange.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items(
            &data,
            DataFormat::String,
            DataType::Exchange,
            &locales,
            &HashMap::new(),
            &res,
            &item_set,
            gamelibs_zip,
        )
        .await
    }

    async fn read_skill_books<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::SkillBook;
            cx.notify();
        });
        let locales = self.read_skill_book_locales(gamedatas_zip).await?;
        let res = self.read_skill_book_itemres(gamedatas_zip).await?;
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemdata_skillbook.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items(
            &data,
            DataFormat::String,
            DataType::SkillBook,
            &locales,
            &HashMap::new(),
            &res,
            &item_set,
            gamelibs_zip,
        )
        .await
    }

    async fn read_gems<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Gem;
            cx.notify();
        });
        let locales = self.read_gem_locales(gamedatas_zip).await?;
        let res = self.read_gem_itemres(gamedatas_zip).await?;
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemdata_enchantstone.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items(
            &data,
            DataFormat::String,
            DataType::Gem,
            &locales,
            &HashMap::new(),
            &res,
            &item_set,
            gamelibs_zip,
        )
        .await
    }

    async fn read_sealed_fellows<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::SealedFellow;
            cx.notify();
        });
        let locales = self.read_sealed_fellow_locales(gamedatas_zip).await?;
        let res = self.read_sealed_fellow_itemres(gamedatas_zip).await?;
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemdata_sealedfellow.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items(
            &data,
            DataFormat::String,
            DataType::SealedFellow,
            &locales,
            &HashMap::new(),
            &res,
            &item_set,
            gamelibs_zip,
        )
        .await
    }

    async fn read_sealed_fellow_locales<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\localized\localstringdata_item_sealedfellow.sxb")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_locale(&data, DataFormat::WideString).await
    }

    async fn read_secondary_weapon_locales<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\localized\localstringdata_item_subitem.sxb")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_locale(&data, DataFormat::WideString).await
    }

    async fn read_gem_locales<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\localized\localstringdata_item_enchantstone.sxb")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_locale(&data, DataFormat::WideString).await
    }
    async fn read_exchange_locales<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\localized\localstringdata_item_exchange.sxb")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_locale(&data, DataFormat::WideString).await
    }

    async fn read_skill_book_locales<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\localized\localstringdata_item_skillbook.sxb")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_locale(&data, DataFormat::WideString).await
    }

    async fn read_recipe_locales<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\localized\localstringdata_item_recipe.sxb")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_locale(&data, DataFormat::WideString).await
    }

    async fn read_consume_locales<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\localized\localstringdata_item_consume.sxb")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_locale(&data, DataFormat::WideString).await
    }
    async fn read_random_box_locales<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\localized\localstringdata_item_randombox.sxb")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_locale(&data, DataFormat::WideString).await
    }

    async fn read_package_locales<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\localized\localstringdata_item_package.sxb")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_locale(&data, DataFormat::WideString).await
    }

    async fn read_style_locales<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\localized\localstringdata_item_style.sxb")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_locale(&data, DataFormat::WideString).await
    }

    async fn read_boost_locales<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\localized\localstringdata_item_boost.sxb")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_locale(&data, DataFormat::WideString).await
    }

        async fn read_bag_locales<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\localized\localstringdata_item_bag.sxb")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_locale(&data, DataFormat::WideString).await
    }

    async fn read_weapon_locales<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\localized\localstringdata_item_weapon.sxb")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_locale(&data, DataFormat::WideString).await
    }

    async fn read_skill_locales<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\localized\localstringdata_skill.sxb")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_locale(&data, DataFormat::WideString).await
    }

    async fn read_accessory_locales<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\localized\localstringdata_item_accessory.sxb")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_locale(&data, DataFormat::WideString).await
    }

    async fn read_material_locales<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\localized\localstringdata_item_material.sxb")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_locale(&data, DataFormat::WideString).await
    }
    async fn read_exchange_itemres<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, ItemRes>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemres_exchange.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_res(&data, DataFormat::String).await
    }
    async fn read_sealed_fellow_itemres<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, ItemRes>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemres_sealedfellow.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_res(&data, DataFormat::String).await
    }

    async fn read_accessory_itemres<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, ItemRes>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemres_accessory.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_res(&data, DataFormat::String).await
    }

    async fn read_material_itemres<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, ItemRes>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemres_material.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_res(&data, DataFormat::String).await
    }

    async fn read_bag_itemres<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, ItemRes>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemres_bag.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_res(&data, DataFormat::String).await
    }

    async fn read_random_box_itemres<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, ItemRes>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemres_randombox.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_res(&data, DataFormat::String).await
    }

    async fn read_style_itemres<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, ItemRes>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemres_style.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_res(&data, DataFormat::String).await
    }

    async fn read_package_itemres<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, ItemRes>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemres_package.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_res(&data, DataFormat::String).await
    }

    async fn read_armor_itemres<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, ItemRes>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemres_armor.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_res(&data, DataFormat::String).await
    }

    async fn read_skill_book_itemres<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, ItemRes>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemres_skillbook.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_res(&data, DataFormat::String).await
    }

    async fn read_gem_itemres<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, ItemRes>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemres_enchantstone.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_res(&data, DataFormat::String).await
    }

    async fn read_fellow_equip_itemres<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, ItemRes>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemres_fellowequip.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_res(&data, DataFormat::String).await
    }

    async fn read_recipe_itemres<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, ItemRes>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemres_recipe.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_res(&data, DataFormat::String).await
    }

    async fn read_boost_itemres<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, ItemRes>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemres_boost.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_res(&data, DataFormat::String).await
    }

    async fn read_consume_itemres<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, ItemRes>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemres_consume.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_res(&data, DataFormat::String).await
    }

    async fn read_weapon_itemres<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, ItemRes>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemres_weapon.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_res(&data, DataFormat::String).await
    }

    async fn read_secondary_weapon_itemres<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, ItemRes>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemres_sub.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_res(&data, DataFormat::String).await
    }

    async fn read_armor_locales<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\localized\localstringdata_item_armor.sxb")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_locale(&data, DataFormat::WideString).await
    }

    async fn read_fellow_equip_locales<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\localized\localstringdata_item_fellowequip.sxb")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_locale(&data, DataFormat::WideString).await
    }

    async fn read_itemset_locales<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\localized\localstringdata_item_setitem.sxb")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_locale(&data, DataFormat::WideString).await
    }

    async fn read_items<T: AbstractItem + Item, R: Read + Seek>(
        &mut self,
        data: &[u8],
        format: DataFormat,
        constructor: fn(T) -> DataType,
        locales: &HashMap<SharedString, Locale>,
        skill_locales: &HashMap<SharedString, Locale>,
        res: &HashMap<SharedString, ItemRes>,
        item_set: &Vec<ItemSet>,
        gamelibs_zip: &mut ZipArchive<R>,
    ) -> Result<()> {
        let cursor = Cursor::new(data);
        let mut reader = BufReader::new(cursor);

        let definitions = self.read_definitions(&mut reader).await?;
        let item_count = self.read_item_count(&mut reader).await?;
        let offsets = self.read_offsets(&mut reader, item_count, definitions.len()).await?;

        let global_offset = reader.stream_position().await?;

        for item_idx in 0..item_count {
            let mut item = T::default()
                .read(&mut reader, &offsets, item_idx, &definitions, global_offset, format)
                .await?;

            item.set_locale(locales, skill_locales);
            item.set_item_set(item_set);
            item.set_product(&self.products_by_recipe_id, &self.products_by_result_id);

            item.set_icon(res, gamelibs_zip).await?;
            let mut c = constructor(item);

            match c {
                DataType::RandomBox(ref mut random_box) => random_box.set_random_box_group(&self.random_box_groups),
                DataType::Package(ref mut package) => package.validate(&self.items),
                _ => {}
            }

            //c.validate_effects();
            //c.validate_grades();
            self.items.insert(c.get_id(), Rc::new(c));
        }

        Ok(())
    }

    async fn read_random_box_group(&mut self, data: &[u8], format: DataFormat) -> Result<()> {
        let cursor = Cursor::new(data);
        let mut reader = BufReader::new(cursor);

        let definitions = self.read_definitions(&mut reader).await?;
        let item_count = self.read_item_count(&mut reader).await?;
        let offsets = self.read_offsets(&mut reader, item_count, definitions.len()).await?;

        let global_offset = reader.stream_position().await?;
        let mut groups = HashMap::with_capacity(item_count);

        for item_idx in 0..item_count {
            let item = RandomBoxGroup::default()
                .read(&mut reader, &offsets, item_idx, &definitions, global_offset, format)
                .await?;
            let id = item.node.id.clone();

            let item = Rc::new(RefCell::new(item));

            groups.insert(id, item.clone());
        }

        self.random_box_groups = groups;
        Ok(())
    }

    async fn read_random_box_probability(&mut self, data: &[u8], format: DataFormat) -> Result<HashMap<SharedString, RandomBoxProbability>> {
        let cursor = Cursor::new(data);
        let mut reader = BufReader::new(cursor);

        let definitions = self.read_definitions(&mut reader).await?;
        let item_count = self.read_item_count(&mut reader).await?;
        let offsets = self.read_offsets(&mut reader, item_count, definitions.len()).await?;

        let global_offset = reader.stream_position().await?;
        let mut groups = HashMap::with_capacity(item_count);

        for item_idx in 0..item_count {
            let item = RandomBoxProbability::default()
                .read(&mut reader, &offsets, item_idx, &definitions, global_offset, format)
                .await?;
            let id = item.randomboxgroupid.clone();

            groups.insert(id, item.clone());
        }

        Ok(groups)
    }

    async fn read_product_material(&mut self, data: &[u8], format: DataFormat) -> Result<()> {
        let cursor = Cursor::new(data);
        let mut reader = BufReader::new(cursor);

        let definitions = self.read_definitions(&mut reader).await?;
        let item_count = self.read_item_count(&mut reader).await?;
        let offsets = self.read_offsets(&mut reader, item_count, definitions.len()).await?;

        let global_offset = reader.stream_position().await?;
        let mut products_by_recipe_id = HashMap::with_capacity(item_count);
        let mut products_by_result_id = HashMap::with_capacity(item_count);
        for item_idx in 0..item_count {
            let item = Product::default()
                .read(&mut reader, &offsets, item_idx, &definitions, global_offset, format)
                .await?;
            let id = item.node.id.clone();

            let productid = item.productid.clone();
            let item = Rc::new(RefCell::new(item));
            products_by_recipe_id.insert(productid, item.clone());
            products_by_result_id.insert(id, item.clone());
        }

        self.products_by_recipe_id = products_by_recipe_id;
        self.products_by_result_id = products_by_result_id;
        Ok(())
    }
