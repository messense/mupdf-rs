#![forbid(unsafe_code)]

#[derive(Clone, Copy, Debug)]
pub struct Font {
    pub name: &'static str,
    pub data: &'static [u8],
    pub index: i32,
}

const NOTOEMOJI_REGULAR: Font = Font {
    name: "Noto Emoji",
    data: include_bytes!("../fonts/NotoEmoji-Regular.ttf"),
    index: 0,
};

const NOTOMUSIC_REGULAR: Font = Font {
    name: "Noto Music",
    data: include_bytes!("../fonts/NotoMusic-Regular.otf"),
    index: 0,
};

const NOTONASKHARABIC_REGULAR: Font = Font {
    name: "Noto Naskh Arabic",
    data: include_bytes!("../fonts/NotoNaskhArabic-Regular.otf"),
    index: 0,
};

const NOTONASTALIQURDU_REGULAR: Font = Font {
    name: "Noto Nastaliq Urdu",
    data: include_bytes!("../fonts/NotoNastaliqUrdu-Regular.otf"),
    index: 0,
};

const NOTOSANS_REGULAR: Font = Font {
    name: "Noto Sans",
    data: include_bytes!("../fonts/NotoSans-Regular.otf"),
    index: 0,
};

const NOTOSANSADLAM_REGULAR: Font = Font {
    name: "Noto Sans Adlam",
    data: include_bytes!("../fonts/NotoSansAdlam-Regular.otf"),
    index: 0,
};

const NOTOSANSANATOLIANHIEROGLYPHS_REGULAR: Font = Font {
    name: "Noto Sans Anatolian Hieroglyphs",
    data: include_bytes!("../fonts/NotoSansAnatolianHieroglyphs-Regular.otf"),
    index: 0,
};

const NOTOSANSAVESTAN_REGULAR: Font = Font {
    name: "Noto Sans Avestan",
    data: include_bytes!("../fonts/NotoSansAvestan-Regular.otf"),
    index: 0,
};

const NOTOSANSBAMUM_REGULAR: Font = Font {
    name: "Noto Sans Bamum",
    data: include_bytes!("../fonts/NotoSansBamum-Regular.otf"),
    index: 0,
};

const NOTOSANSBASSAVAH_REGULAR: Font = Font {
    name: "Noto Sans Bassa Vah",
    data: include_bytes!("../fonts/NotoSansBassaVah-Regular.otf"),
    index: 0,
};

const NOTOSANSBATAK_REGULAR: Font = Font {
    name: "Noto Sans Batak",
    data: include_bytes!("../fonts/NotoSansBatak-Regular.otf"),
    index: 0,
};

const NOTOSANSBHAIKSUKI_REGULAR: Font = Font {
    name: "Noto Sans Bhaiksuki",
    data: include_bytes!("../fonts/NotoSansBhaiksuki-Regular.otf"),
    index: 0,
};

const NOTOSANSBRAHMI_REGULAR: Font = Font {
    name: "Noto Sans Brahmi",
    data: include_bytes!("../fonts/NotoSansBrahmi-Regular.otf"),
    index: 0,
};

const NOTOSANSBUGINESE_REGULAR: Font = Font {
    name: "Noto Sans Buginese",
    data: include_bytes!("../fonts/NotoSansBuginese-Regular.otf"),
    index: 0,
};

const NOTOSANSBUHID_REGULAR: Font = Font {
    name: "Noto Sans Buhid",
    data: include_bytes!("../fonts/NotoSansBuhid-Regular.otf"),
    index: 0,
};

const NOTOSANSCANADIANABORIGINAL_REGULAR: Font = Font {
    name: "Noto Sans Canadian Aboriginal",
    data: include_bytes!("../fonts/NotoSansCanadianAboriginal-Regular.otf"),
    index: 0,
};

const NOTOSANSCARIAN_REGULAR: Font = Font {
    name: "Noto Sans Carian",
    data: include_bytes!("../fonts/NotoSansCarian-Regular.otf"),
    index: 0,
};

const NOTOSANSCAUCASIANALBANIAN_REGULAR: Font = Font {
    name: "Noto Sans Caucasian Albanian",
    data: include_bytes!("../fonts/NotoSansCaucasianAlbanian-Regular.otf"),
    index: 0,
};

const NOTOSANSCHAKMA_REGULAR: Font = Font {
    name: "Noto Sans Chakma",
    data: include_bytes!("../fonts/NotoSansChakma-Regular.otf"),
    index: 0,
};

const NOTOSANSCHAM_REGULAR: Font = Font {
    name: "Noto Sans Cham",
    data: include_bytes!("../fonts/NotoSansCham-Regular.otf"),
    index: 0,
};

const NOTOSANSCHEROKEE_REGULAR: Font = Font {
    name: "Noto Sans Cherokee",
    data: include_bytes!("../fonts/NotoSansCherokee-Regular.otf"),
    index: 0,
};

const NOTOSANSCHORASMIAN_REGULAR: Font = Font {
    name: "Noto Sans Chorasmian",
    data: include_bytes!("../fonts/NotoSansChorasmian-Regular.otf"),
    index: 0,
};

const NOTOSANSCOPTIC_REGULAR: Font = Font {
    name: "Noto Sans Coptic",
    data: include_bytes!("../fonts/NotoSansCoptic-Regular.otf"),
    index: 0,
};

const NOTOSANSCUNEIFORM_REGULAR: Font = Font {
    name: "Noto Sans Cuneiform",
    data: include_bytes!("../fonts/NotoSansCuneiform-Regular.otf"),
    index: 0,
};

const NOTOSANSCYPRIOT_REGULAR: Font = Font {
    name: "Noto Sans Cypriot",
    data: include_bytes!("../fonts/NotoSansCypriot-Regular.otf"),
    index: 0,
};

const NOTOSANSCYPROMINOAN_REGULAR: Font = Font {
    name: "Noto Sans Cypro Minoan",
    data: include_bytes!("../fonts/NotoSansCyproMinoan-Regular.otf"),
    index: 0,
};

const NOTOSANSDESERET_REGULAR: Font = Font {
    name: "Noto Sans Deseret",
    data: include_bytes!("../fonts/NotoSansDeseret-Regular.otf"),
    index: 0,
};

const NOTOSANSDUPLOYAN_REGULAR: Font = Font {
    name: "Noto Sans Duployan",
    data: include_bytes!("../fonts/NotoSansDuployan-Regular.otf"),
    index: 0,
};

const NOTOSANSEGYPTIANHIEROGLYPHS_REGULAR: Font = Font {
    name: "Noto Sans Egyptian Hieroglyphs",
    data: include_bytes!("../fonts/NotoSansEgyptianHieroglyphs-Regular.otf"),
    index: 0,
};

const NOTOSANSELBASAN_REGULAR: Font = Font {
    name: "Noto Sans Elbasan",
    data: include_bytes!("../fonts/NotoSansElbasan-Regular.otf"),
    index: 0,
};

const NOTOSANSELYMAIC_REGULAR: Font = Font {
    name: "Noto Sans Elymaic",
    data: include_bytes!("../fonts/NotoSansElymaic-Regular.otf"),
    index: 0,
};

const NOTOSANSGLAGOLITIC_REGULAR: Font = Font {
    name: "Noto Sans Glagolitic",
    data: include_bytes!("../fonts/NotoSansGlagolitic-Regular.otf"),
    index: 0,
};

const NOTOSANSGOTHIC_REGULAR: Font = Font {
    name: "Noto Sans Gothic",
    data: include_bytes!("../fonts/NotoSansGothic-Regular.otf"),
    index: 0,
};

const NOTOSANSGUNJALAGONDI_REGULAR: Font = Font {
    name: "Noto Sans Gunjala Gondi",
    data: include_bytes!("../fonts/NotoSansGunjalaGondi-Regular.otf"),
    index: 0,
};

const NOTOSANSHANIFIROHINGYA_REGULAR: Font = Font {
    name: "Noto Sans Hanifi Rohingya",
    data: include_bytes!("../fonts/NotoSansHanifiRohingya-Regular.otf"),
    index: 0,
};

const NOTOSANSHANUNOO_REGULAR: Font = Font {
    name: "Noto Sans Hanunoo",
    data: include_bytes!("../fonts/NotoSansHanunoo-Regular.otf"),
    index: 0,
};

const NOTOSANSHATRAN_REGULAR: Font = Font {
    name: "Noto Sans Hatran",
    data: include_bytes!("../fonts/NotoSansHatran-Regular.otf"),
    index: 0,
};

const NOTOSANSIMPERIALARAMAIC_REGULAR: Font = Font {
    name: "Noto Sans Imperial Aramaic",
    data: include_bytes!("../fonts/NotoSansImperialAramaic-Regular.otf"),
    index: 0,
};

const NOTOSANSINSCRIPTIONALPAHLAVI_REGULAR: Font = Font {
    name: "Noto Sans Inscriptional Pahlavi",
    data: include_bytes!("../fonts/NotoSansInscriptionalPahlavi-Regular.otf"),
    index: 0,
};

const NOTOSANSINSCRIPTIONALPARTHIAN_REGULAR: Font = Font {
    name: "Noto Sans Inscriptional Parthian",
    data: include_bytes!("../fonts/NotoSansInscriptionalParthian-Regular.otf"),
    index: 0,
};

const NOTOSANSJAVANESE_REGULAR: Font = Font {
    name: "Noto Sans Javanese",
    data: include_bytes!("../fonts/NotoSansJavanese-Regular.otf"),
    index: 0,
};

const NOTOSANSKAITHI_REGULAR: Font = Font {
    name: "Noto Sans Kaithi",
    data: include_bytes!("../fonts/NotoSansKaithi-Regular.otf"),
    index: 0,
};

const NOTOSANSKAWI_REGULAR: Font = Font {
    name: "Noto Sans Kawi",
    data: include_bytes!("../fonts/NotoSansKawi-Regular.otf"),
    index: 0,
};

const NOTOSANSKAYAHLI_REGULAR: Font = Font {
    name: "Noto Sans Kayah Li",
    data: include_bytes!("../fonts/NotoSansKayahLi-Regular.otf"),
    index: 0,
};

const NOTOSANSKHAROSHTHI_REGULAR: Font = Font {
    name: "Noto Sans Kharoshthi",
    data: include_bytes!("../fonts/NotoSansKharoshthi-Regular.otf"),
    index: 0,
};

const NOTOSANSKHUDAWADI_REGULAR: Font = Font {
    name: "Noto Sans Khudawadi",
    data: include_bytes!("../fonts/NotoSansKhudawadi-Regular.otf"),
    index: 0,
};

const NOTOSANSLEPCHA_REGULAR: Font = Font {
    name: "Noto Sans Lepcha",
    data: include_bytes!("../fonts/NotoSansLepcha-Regular.otf"),
    index: 0,
};

const NOTOSANSLIMBU_REGULAR: Font = Font {
    name: "Noto Sans Limbu",
    data: include_bytes!("../fonts/NotoSansLimbu-Regular.otf"),
    index: 0,
};

const NOTOSANSLINEARA_REGULAR: Font = Font {
    name: "Noto Sans Linear A",
    data: include_bytes!("../fonts/NotoSansLinearA-Regular.otf"),
    index: 0,
};

const NOTOSANSLINEARB_REGULAR: Font = Font {
    name: "Noto Sans Linear B",
    data: include_bytes!("../fonts/NotoSansLinearB-Regular.otf"),
    index: 0,
};

const NOTOSANSLISU_REGULAR: Font = Font {
    name: "Noto Sans Lisu",
    data: include_bytes!("../fonts/NotoSansLisu-Regular.otf"),
    index: 0,
};

const NOTOSANSLYCIAN_REGULAR: Font = Font {
    name: "Noto Sans Lycian",
    data: include_bytes!("../fonts/NotoSansLycian-Regular.otf"),
    index: 0,
};

const NOTOSANSLYDIAN_REGULAR: Font = Font {
    name: "Noto Sans Lydian",
    data: include_bytes!("../fonts/NotoSansLydian-Regular.otf"),
    index: 0,
};

const NOTOSANSMAHAJANI_REGULAR: Font = Font {
    name: "Noto Sans Mahajani",
    data: include_bytes!("../fonts/NotoSansMahajani-Regular.otf"),
    index: 0,
};

const NOTOSANSMANDAIC_REGULAR: Font = Font {
    name: "Noto Sans Mandaic",
    data: include_bytes!("../fonts/NotoSansMandaic-Regular.otf"),
    index: 0,
};

const NOTOSANSMANICHAEAN_REGULAR: Font = Font {
    name: "Noto Sans Manichaean",
    data: include_bytes!("../fonts/NotoSansManichaean-Regular.otf"),
    index: 0,
};

const NOTOSANSMARCHEN_REGULAR: Font = Font {
    name: "Noto Sans Marchen",
    data: include_bytes!("../fonts/NotoSansMarchen-Regular.otf"),
    index: 0,
};

const NOTOSANSMASARAMGONDI_REGULAR: Font = Font {
    name: "Noto Sans Masaram Gondi",
    data: include_bytes!("../fonts/NotoSansMasaramGondi-Regular.otf"),
    index: 0,
};

const NOTOSANSMATH_REGULAR: Font = Font {
    name: "Noto Sans Math",
    data: include_bytes!("../fonts/NotoSansMath-Regular.otf"),
    index: 0,
};

const NOTOSANSMEDEFAIDRIN_REGULAR: Font = Font {
    name: "Noto Sans Medefaidrin",
    data: include_bytes!("../fonts/NotoSansMedefaidrin-Regular.otf"),
    index: 0,
};

const NOTOSANSMEETEIMAYEK_REGULAR: Font = Font {
    name: "Noto Sans Meetei Mayek",
    data: include_bytes!("../fonts/NotoSansMeeteiMayek-Regular.otf"),
    index: 0,
};

const NOTOSANSMENDEKIKAKUI_REGULAR: Font = Font {
    name: "Noto Sans Mende Kikakui",
    data: include_bytes!("../fonts/NotoSansMendeKikakui-Regular.otf"),
    index: 0,
};

const NOTOSANSMEROITIC_REGULAR: Font = Font {
    name: "Noto Sans Meroitic",
    data: include_bytes!("../fonts/NotoSansMeroitic-Regular.otf"),
    index: 0,
};

const NOTOSANSMIAO_REGULAR: Font = Font {
    name: "Noto Sans Miao",
    data: include_bytes!("../fonts/NotoSansMiao-Regular.otf"),
    index: 0,
};

const NOTOSANSMODI_REGULAR: Font = Font {
    name: "Noto Sans Modi",
    data: include_bytes!("../fonts/NotoSansModi-Regular.otf"),
    index: 0,
};

const NOTOSANSMONGOLIAN_REGULAR: Font = Font {
    name: "Noto Sans Mongolian",
    data: include_bytes!("../fonts/NotoSansMongolian-Regular.otf"),
    index: 0,
};

const NOTOSANSMRO_REGULAR: Font = Font {
    name: "Noto Sans Mro",
    data: include_bytes!("../fonts/NotoSansMro-Regular.otf"),
    index: 0,
};

const NOTOSANSMULTANI_REGULAR: Font = Font {
    name: "Noto Sans Multani",
    data: include_bytes!("../fonts/NotoSansMultani-Regular.otf"),
    index: 0,
};

const NOTOSANSNKO_REGULAR: Font = Font {
    name: "Noto Sans N Ko",
    data: include_bytes!("../fonts/NotoSansNKo-Regular.otf"),
    index: 0,
};

const NOTOSANSNABATAEAN_REGULAR: Font = Font {
    name: "Noto Sans Nabataean",
    data: include_bytes!("../fonts/NotoSansNabataean-Regular.otf"),
    index: 0,
};

const NOTOSANSNAGMUNDARI_REGULAR: Font = Font {
    name: "Noto Sans Nag Mundari",
    data: include_bytes!("../fonts/NotoSansNagMundari-Regular.otf"),
    index: 0,
};

const NOTOSANSNANDINAGARI_REGULAR: Font = Font {
    name: "Noto Sans Nandinagari",
    data: include_bytes!("../fonts/NotoSansNandinagari-Regular.otf"),
    index: 0,
};

const NOTOSANSNEWTAILUE_REGULAR: Font = Font {
    name: "Noto Sans New Tai Lue",
    data: include_bytes!("../fonts/NotoSansNewTaiLue-Regular.otf"),
    index: 0,
};

const NOTOSANSNEWA_REGULAR: Font = Font {
    name: "Noto Sans Newa",
    data: include_bytes!("../fonts/NotoSansNewa-Regular.otf"),
    index: 0,
};

const NOTOSANSNUSHU_REGULAR: Font = Font {
    name: "Noto Sans Nushu",
    data: include_bytes!("../fonts/NotoSansNushu-Regular.otf"),
    index: 0,
};

const NOTOSANSOGHAM_REGULAR: Font = Font {
    name: "Noto Sans Ogham",
    data: include_bytes!("../fonts/NotoSansOgham-Regular.otf"),
    index: 0,
};

const NOTOSANSOLCHIKI_REGULAR: Font = Font {
    name: "Noto Sans Ol Chiki",
    data: include_bytes!("../fonts/NotoSansOlChiki-Regular.otf"),
    index: 0,
};

const NOTOSANSOLDHUNGARIAN_REGULAR: Font = Font {
    name: "Noto Sans Old Hungarian",
    data: include_bytes!("../fonts/NotoSansOldHungarian-Regular.otf"),
    index: 0,
};

const NOTOSANSOLDITALIC_REGULAR: Font = Font {
    name: "Noto Sans Old Italic",
    data: include_bytes!("../fonts/NotoSansOldItalic-Regular.otf"),
    index: 0,
};

const NOTOSANSOLDNORTHARABIAN_REGULAR: Font = Font {
    name: "Noto Sans Old North Arabian",
    data: include_bytes!("../fonts/NotoSansOldNorthArabian-Regular.otf"),
    index: 0,
};

const NOTOSANSOLDPERMIC_REGULAR: Font = Font {
    name: "Noto Sans Old Permic",
    data: include_bytes!("../fonts/NotoSansOldPermic-Regular.otf"),
    index: 0,
};

const NOTOSANSOLDPERSIAN_REGULAR: Font = Font {
    name: "Noto Sans Old Persian",
    data: include_bytes!("../fonts/NotoSansOldPersian-Regular.otf"),
    index: 0,
};

const NOTOSANSOLDSOGDIAN_REGULAR: Font = Font {
    name: "Noto Sans Old Sogdian",
    data: include_bytes!("../fonts/NotoSansOldSogdian-Regular.otf"),
    index: 0,
};

const NOTOSANSOLDSOUTHARABIAN_REGULAR: Font = Font {
    name: "Noto Sans Old South Arabian",
    data: include_bytes!("../fonts/NotoSansOldSouthArabian-Regular.otf"),
    index: 0,
};

const NOTOSANSOLDTURKIC_REGULAR: Font = Font {
    name: "Noto Sans Old Turkic",
    data: include_bytes!("../fonts/NotoSansOldTurkic-Regular.otf"),
    index: 0,
};

const NOTOSANSOSAGE_REGULAR: Font = Font {
    name: "Noto Sans Osage",
    data: include_bytes!("../fonts/NotoSansOsage-Regular.otf"),
    index: 0,
};

const NOTOSANSOSMANYA_REGULAR: Font = Font {
    name: "Noto Sans Osmanya",
    data: include_bytes!("../fonts/NotoSansOsmanya-Regular.otf"),
    index: 0,
};

const NOTOSANSPAHAWHHMONG_REGULAR: Font = Font {
    name: "Noto Sans Pahawh Hmong",
    data: include_bytes!("../fonts/NotoSansPahawhHmong-Regular.otf"),
    index: 0,
};

const NOTOSANSPALMYRENE_REGULAR: Font = Font {
    name: "Noto Sans Palmyrene",
    data: include_bytes!("../fonts/NotoSansPalmyrene-Regular.otf"),
    index: 0,
};

const NOTOSANSPAUCINHAU_REGULAR: Font = Font {
    name: "Noto Sans Pau Cin Hau",
    data: include_bytes!("../fonts/NotoSansPauCinHau-Regular.otf"),
    index: 0,
};

const NOTOSANSPHAGSPA_REGULAR: Font = Font {
    name: "Noto Sans Phags Pa",
    data: include_bytes!("../fonts/NotoSansPhagsPa-Regular.otf"),
    index: 0,
};

const NOTOSANSPHOENICIAN_REGULAR: Font = Font {
    name: "Noto Sans Phoenician",
    data: include_bytes!("../fonts/NotoSansPhoenician-Regular.otf"),
    index: 0,
};

const NOTOSANSPSALTERPAHLAVI_REGULAR: Font = Font {
    name: "Noto Sans Psalter Pahlavi",
    data: include_bytes!("../fonts/NotoSansPsalterPahlavi-Regular.otf"),
    index: 0,
};

const NOTOSANSREJANG_REGULAR: Font = Font {
    name: "Noto Sans Rejang",
    data: include_bytes!("../fonts/NotoSansRejang-Regular.otf"),
    index: 0,
};

const NOTOSANSRUNIC_REGULAR: Font = Font {
    name: "Noto Sans Runic",
    data: include_bytes!("../fonts/NotoSansRunic-Regular.otf"),
    index: 0,
};

const NOTOSANSSAMARITAN_REGULAR: Font = Font {
    name: "Noto Sans Samaritan",
    data: include_bytes!("../fonts/NotoSansSamaritan-Regular.otf"),
    index: 0,
};

const NOTOSANSSAURASHTRA_REGULAR: Font = Font {
    name: "Noto Sans Saurashtra",
    data: include_bytes!("../fonts/NotoSansSaurashtra-Regular.otf"),
    index: 0,
};

const NOTOSANSSHARADA_REGULAR: Font = Font {
    name: "Noto Sans Sharada",
    data: include_bytes!("../fonts/NotoSansSharada-Regular.otf"),
    index: 0,
};

const NOTOSANSSHAVIAN_REGULAR: Font = Font {
    name: "Noto Sans Shavian",
    data: include_bytes!("../fonts/NotoSansShavian-Regular.otf"),
    index: 0,
};

const NOTOSANSSIDDHAM_REGULAR: Font = Font {
    name: "Noto Sans Siddham",
    data: include_bytes!("../fonts/NotoSansSiddham-Regular.otf"),
    index: 0,
};

const NOTOSANSSIGNWRITING_REGULAR: Font = Font {
    name: "Noto Sans Sign Writing",
    data: include_bytes!("../fonts/NotoSansSignWriting-Regular.otf"),
    index: 0,
};

const NOTOSANSSOGDIAN_REGULAR: Font = Font {
    name: "Noto Sans Sogdian",
    data: include_bytes!("../fonts/NotoSansSogdian-Regular.otf"),
    index: 0,
};

const NOTOSANSSORASOMPENG_REGULAR: Font = Font {
    name: "Noto Sans Sora Sompeng",
    data: include_bytes!("../fonts/NotoSansSoraSompeng-Regular.otf"),
    index: 0,
};

const NOTOSANSSOYOMBO_REGULAR: Font = Font {
    name: "Noto Sans Soyombo",
    data: include_bytes!("../fonts/NotoSansSoyombo-Regular.otf"),
    index: 0,
};

const NOTOSANSSUNDANESE_REGULAR: Font = Font {
    name: "Noto Sans Sundanese",
    data: include_bytes!("../fonts/NotoSansSundanese-Regular.otf"),
    index: 0,
};

const NOTOSANSSYLOTINAGRI_REGULAR: Font = Font {
    name: "Noto Sans Syloti Nagri",
    data: include_bytes!("../fonts/NotoSansSylotiNagri-Regular.otf"),
    index: 0,
};

const NOTOSANSSYMBOLS_REGULAR: Font = Font {
    name: "Noto Sans Symbols",
    data: include_bytes!("../fonts/NotoSansSymbols-Regular.otf"),
    index: 0,
};

const NOTOSANSSYMBOLS2_REGULAR: Font = Font {
    name: "Noto Sans Symbols 2",
    data: include_bytes!("../fonts/NotoSansSymbols2-Regular.otf"),
    index: 0,
};

const NOTOSANSSYRIAC_REGULAR: Font = Font {
    name: "Noto Sans Syriac",
    data: include_bytes!("../fonts/NotoSansSyriac-Regular.otf"),
    index: 0,
};

const NOTOSANSTAGALOG_REGULAR: Font = Font {
    name: "Noto Sans Tagalog",
    data: include_bytes!("../fonts/NotoSansTagalog-Regular.otf"),
    index: 0,
};

const NOTOSANSTAGBANWA_REGULAR: Font = Font {
    name: "Noto Sans Tagbanwa",
    data: include_bytes!("../fonts/NotoSansTagbanwa-Regular.otf"),
    index: 0,
};

const NOTOSANSTAILE_REGULAR: Font = Font {
    name: "Noto Sans Tai Le",
    data: include_bytes!("../fonts/NotoSansTaiLe-Regular.otf"),
    index: 0,
};

const NOTOSANSTAITHAM_REGULAR: Font = Font {
    name: "Noto Sans Tai Tham",
    data: include_bytes!("../fonts/NotoSansTaiTham-Regular.otf"),
    index: 0,
};

const NOTOSANSTAIVIET_REGULAR: Font = Font {
    name: "Noto Sans Tai Viet",
    data: include_bytes!("../fonts/NotoSansTaiViet-Regular.otf"),
    index: 0,
};

const NOTOSANSTAKRI_REGULAR: Font = Font {
    name: "Noto Sans Takri",
    data: include_bytes!("../fonts/NotoSansTakri-Regular.otf"),
    index: 0,
};

const NOTOSANSTANGSA_REGULAR: Font = Font {
    name: "Noto Sans Tangsa",
    data: include_bytes!("../fonts/NotoSansTangsa-Regular.otf"),
    index: 0,
};

const NOTOSANSTHAANA_REGULAR: Font = Font {
    name: "Noto Sans Thaana",
    data: include_bytes!("../fonts/NotoSansThaana-Regular.otf"),
    index: 0,
};

const NOTOSANSTIFINAGH_REGULAR: Font = Font {
    name: "Noto Sans Tifinagh",
    data: include_bytes!("../fonts/NotoSansTifinagh-Regular.otf"),
    index: 0,
};

const NOTOSANSTIRHUTA_REGULAR: Font = Font {
    name: "Noto Sans Tirhuta",
    data: include_bytes!("../fonts/NotoSansTirhuta-Regular.otf"),
    index: 0,
};

const NOTOSANSUGARITIC_REGULAR: Font = Font {
    name: "Noto Sans Ugaritic",
    data: include_bytes!("../fonts/NotoSansUgaritic-Regular.otf"),
    index: 0,
};

const NOTOSANSVAI_REGULAR: Font = Font {
    name: "Noto Sans Vai",
    data: include_bytes!("../fonts/NotoSansVai-Regular.otf"),
    index: 0,
};

const NOTOSANSWANCHO_REGULAR: Font = Font {
    name: "Noto Sans Wancho",
    data: include_bytes!("../fonts/NotoSansWancho-Regular.otf"),
    index: 0,
};

const NOTOSANSWARANGCITI_REGULAR: Font = Font {
    name: "Noto Sans Warang Citi",
    data: include_bytes!("../fonts/NotoSansWarangCiti-Regular.otf"),
    index: 0,
};

const NOTOSANSYI_REGULAR: Font = Font {
    name: "Noto Sans Yi",
    data: include_bytes!("../fonts/NotoSansYi-Regular.otf"),
    index: 0,
};

const NOTOSANSZANABAZARSQUARE_REGULAR: Font = Font {
    name: "Noto Sans Zanabazar Square",
    data: include_bytes!("../fonts/NotoSansZanabazarSquare-Regular.otf"),
    index: 0,
};

const NOTOSERIF_REGULAR: Font = Font {
    name: "Noto Serif",
    data: include_bytes!("../fonts/NotoSerif-Regular.otf"),
    index: 0,
};

const NOTOSERIFAHOM_REGULAR: Font = Font {
    name: "Noto Serif Ahom",
    data: include_bytes!("../fonts/NotoSerifAhom-Regular.otf"),
    index: 0,
};

const NOTOSERIFARMENIAN_REGULAR: Font = Font {
    name: "Noto Serif Armenian",
    data: include_bytes!("../fonts/NotoSerifArmenian-Regular.otf"),
    index: 0,
};

const NOTOSERIFBALINESE_REGULAR: Font = Font {
    name: "Noto Serif Balinese",
    data: include_bytes!("../fonts/NotoSerifBalinese-Regular.otf"),
    index: 0,
};

const NOTOSERIFBENGALI_REGULAR: Font = Font {
    name: "Noto Serif Bengali",
    data: include_bytes!("../fonts/NotoSerifBengali-Regular.otf"),
    index: 0,
};

const NOTOSERIFDEVANAGARI_REGULAR: Font = Font {
    name: "Noto Serif Devanagari",
    data: include_bytes!("../fonts/NotoSerifDevanagari-Regular.otf"),
    index: 0,
};

const NOTOSERIFDIVESAKURU_REGULAR: Font = Font {
    name: "Noto Serif Dives Akuru",
    data: include_bytes!("../fonts/NotoSerifDivesAkuru-Regular.otf"),
    index: 0,
};

const NOTOSERIFDOGRA_REGULAR: Font = Font {
    name: "Noto Serif Dogra",
    data: include_bytes!("../fonts/NotoSerifDogra-Regular.otf"),
    index: 0,
};

const NOTOSERIFETHIOPIC_REGULAR: Font = Font {
    name: "Noto Serif Ethiopic",
    data: include_bytes!("../fonts/NotoSerifEthiopic-Regular.otf"),
    index: 0,
};

const NOTOSERIFGEORGIAN_REGULAR: Font = Font {
    name: "Noto Serif Georgian",
    data: include_bytes!("../fonts/NotoSerifGeorgian-Regular.otf"),
    index: 0,
};

const NOTOSERIFGRANTHA_REGULAR: Font = Font {
    name: "Noto Serif Grantha",
    data: include_bytes!("../fonts/NotoSerifGrantha-Regular.otf"),
    index: 0,
};

const NOTOSERIFGUJARATI_REGULAR: Font = Font {
    name: "Noto Serif Gujarati",
    data: include_bytes!("../fonts/NotoSerifGujarati-Regular.otf"),
    index: 0,
};

const NOTOSERIFGURMUKHI_REGULAR: Font = Font {
    name: "Noto Serif Gurmukhi",
    data: include_bytes!("../fonts/NotoSerifGurmukhi-Regular.otf"),
    index: 0,
};

const NOTOSERIFHEBREW_REGULAR: Font = Font {
    name: "Noto Serif Hebrew",
    data: include_bytes!("../fonts/NotoSerifHebrew-Regular.otf"),
    index: 0,
};

const NOTOSERIFKANNADA_REGULAR: Font = Font {
    name: "Noto Serif Kannada",
    data: include_bytes!("../fonts/NotoSerifKannada-Regular.otf"),
    index: 0,
};

const NOTOSERIFKHITANSMALLSCRIPT_REGULAR: Font = Font {
    name: "Noto Serif Khitan Small Script",
    data: include_bytes!("../fonts/NotoSerifKhitanSmallScript-Regular.otf"),
    index: 0,
};

const NOTOSERIFKHMER_REGULAR: Font = Font {
    name: "Noto Serif Khmer",
    data: include_bytes!("../fonts/NotoSerifKhmer-Regular.otf"),
    index: 0,
};

const NOTOSERIFKHOJKI_REGULAR: Font = Font {
    name: "Noto Serif Khojki",
    data: include_bytes!("../fonts/NotoSerifKhojki-Regular.otf"),
    index: 0,
};

const NOTOSERIFLAO_REGULAR: Font = Font {
    name: "Noto Serif Lao",
    data: include_bytes!("../fonts/NotoSerifLao-Regular.otf"),
    index: 0,
};

const NOTOSERIFMAKASAR_REGULAR: Font = Font {
    name: "Noto Serif Makasar",
    data: include_bytes!("../fonts/NotoSerifMakasar-Regular.otf"),
    index: 0,
};

const NOTOSERIFMALAYALAM_REGULAR: Font = Font {
    name: "Noto Serif Malayalam",
    data: include_bytes!("../fonts/NotoSerifMalayalam-Regular.otf"),
    index: 0,
};

const NOTOSERIFMYANMAR_REGULAR: Font = Font {
    name: "Noto Serif Myanmar",
    data: include_bytes!("../fonts/NotoSerifMyanmar-Regular.otf"),
    index: 0,
};

const NOTOSERIFNYIAKENGPUACHUEHMONG_REGULAR: Font = Font {
    name: "Noto Serif Nyiakeng Puachue Hmong",
    data: include_bytes!("../fonts/NotoSerifNyiakengPuachueHmong-Regular.otf"),
    index: 0,
};

const NOTOSERIFOLDUYGHUR_REGULAR: Font = Font {
    name: "Noto Serif Old Uyghur",
    data: include_bytes!("../fonts/NotoSerifOldUyghur-Regular.otf"),
    index: 0,
};

const NOTOSERIFORIYA_REGULAR: Font = Font {
    name: "Noto Serif Oriya",
    data: include_bytes!("../fonts/NotoSerifOriya-Regular.otf"),
    index: 0,
};

const NOTOSERIFSINHALA_REGULAR: Font = Font {
    name: "Noto Serif Sinhala",
    data: include_bytes!("../fonts/NotoSerifSinhala-Regular.otf"),
    index: 0,
};

const NOTOSERIFTAMIL_REGULAR: Font = Font {
    name: "Noto Serif Tamil",
    data: include_bytes!("../fonts/NotoSerifTamil-Regular.otf"),
    index: 0,
};

const NOTOSERIFTANGUT_REGULAR: Font = Font {
    name: "Noto Serif Tangut",
    data: include_bytes!("../fonts/NotoSerifTangut-Regular.otf"),
    index: 0,
};

const NOTOSERIFTELUGU_REGULAR: Font = Font {
    name: "Noto Serif Telugu",
    data: include_bytes!("../fonts/NotoSerifTelugu-Regular.otf"),
    index: 0,
};

const NOTOSERIFTHAI_REGULAR: Font = Font {
    name: "Noto Serif Thai",
    data: include_bytes!("../fonts/NotoSerifThai-Regular.otf"),
    index: 0,
};

const NOTOSERIFTIBETAN_REGULAR: Font = Font {
    name: "Noto Serif Tibetan",
    data: include_bytes!("../fonts/NotoSerifTibetan-Regular.otf"),
    index: 0,
};

const NOTOSERIFTOTO_REGULAR: Font = Font {
    name: "Noto Serif Toto",
    data: include_bytes!("../fonts/NotoSerifToto-Regular.otf"),
    index: 0,
};

const NOTOSERIFVITHKUQI_REGULAR: Font = Font {
    name: "Noto Serif Vithkuqi",
    data: include_bytes!("../fonts/NotoSerifVithkuqi-Regular.otf"),
    index: 0,
};

const NOTOSERIFYEZIDI_REGULAR: Font = Font {
    name: "Noto Serif Yezidi",
    data: include_bytes!("../fonts/NotoSerifYezidi-Regular.otf"),
    index: 0,
};

#[derive(Clone, Copy)]
struct Entry {
    stem: &'static str,
    serif: bool,
    postscript_name: &'static str,
    font: Font,
}

const FONTS: &[Entry] = &[
    Entry {
        stem: "Emoji",
        serif: false,
        postscript_name: "NotoEmoji",
        font: NOTOEMOJI_REGULAR,
    },
    Entry {
        stem: "Music",
        serif: false,
        postscript_name: "NotoMusic",
        font: NOTOMUSIC_REGULAR,
    },
    Entry {
        stem: "Naskh",
        serif: false,
        postscript_name: "NotoNaskhArabic",
        font: NOTONASKHARABIC_REGULAR,
    },
    Entry {
        stem: "NastaliqUrdu",
        serif: false,
        postscript_name: "NotoNastaliqUrdu",
        font: NOTONASTALIQURDU_REGULAR,
    },
    Entry {
        stem: "",
        serif: false,
        postscript_name: "NotoSans",
        font: NOTOSANS_REGULAR,
    },
    Entry {
        stem: "Adlam",
        serif: false,
        postscript_name: "NotoSansAdlam",
        font: NOTOSANSADLAM_REGULAR,
    },
    Entry {
        stem: "AnatolianHieroglyphs",
        serif: false,
        postscript_name: "NotoSansAnatolianHieroglyphs",
        font: NOTOSANSANATOLIANHIEROGLYPHS_REGULAR,
    },
    Entry {
        stem: "Avestan",
        serif: false,
        postscript_name: "NotoSansAvestan",
        font: NOTOSANSAVESTAN_REGULAR,
    },
    Entry {
        stem: "Bamum",
        serif: false,
        postscript_name: "NotoSansBamum",
        font: NOTOSANSBAMUM_REGULAR,
    },
    Entry {
        stem: "BassaVah",
        serif: false,
        postscript_name: "NotoSansBassaVah",
        font: NOTOSANSBASSAVAH_REGULAR,
    },
    Entry {
        stem: "Batak",
        serif: false,
        postscript_name: "NotoSansBatak",
        font: NOTOSANSBATAK_REGULAR,
    },
    Entry {
        stem: "Bhaiksuki",
        serif: false,
        postscript_name: "NotoSansBhaiksuki",
        font: NOTOSANSBHAIKSUKI_REGULAR,
    },
    Entry {
        stem: "Brahmi",
        serif: false,
        postscript_name: "NotoSansBrahmi",
        font: NOTOSANSBRAHMI_REGULAR,
    },
    Entry {
        stem: "Buginese",
        serif: false,
        postscript_name: "NotoSansBuginese",
        font: NOTOSANSBUGINESE_REGULAR,
    },
    Entry {
        stem: "Buhid",
        serif: false,
        postscript_name: "NotoSansBuhid",
        font: NOTOSANSBUHID_REGULAR,
    },
    Entry {
        stem: "CanadianAboriginal",
        serif: false,
        postscript_name: "NotoSansCanadianAboriginal",
        font: NOTOSANSCANADIANABORIGINAL_REGULAR,
    },
    Entry {
        stem: "Carian",
        serif: false,
        postscript_name: "NotoSansCarian",
        font: NOTOSANSCARIAN_REGULAR,
    },
    Entry {
        stem: "CaucasianAlbanian",
        serif: false,
        postscript_name: "NotoSansCaucasianAlbanian",
        font: NOTOSANSCAUCASIANALBANIAN_REGULAR,
    },
    Entry {
        stem: "Chakma",
        serif: false,
        postscript_name: "NotoSansChakma",
        font: NOTOSANSCHAKMA_REGULAR,
    },
    Entry {
        stem: "Cham",
        serif: false,
        postscript_name: "NotoSansCham",
        font: NOTOSANSCHAM_REGULAR,
    },
    Entry {
        stem: "Cherokee",
        serif: false,
        postscript_name: "NotoSansCherokee",
        font: NOTOSANSCHEROKEE_REGULAR,
    },
    Entry {
        stem: "Chorasmian",
        serif: false,
        postscript_name: "NotoSansChorasmian",
        font: NOTOSANSCHORASMIAN_REGULAR,
    },
    Entry {
        stem: "Coptic",
        serif: false,
        postscript_name: "NotoSansCoptic",
        font: NOTOSANSCOPTIC_REGULAR,
    },
    Entry {
        stem: "Cuneiform",
        serif: false,
        postscript_name: "NotoSansCuneiform",
        font: NOTOSANSCUNEIFORM_REGULAR,
    },
    Entry {
        stem: "Cypriot",
        serif: false,
        postscript_name: "NotoSansCypriot",
        font: NOTOSANSCYPRIOT_REGULAR,
    },
    Entry {
        stem: "CyproMinoan",
        serif: false,
        postscript_name: "NotoSansCyproMinoan",
        font: NOTOSANSCYPROMINOAN_REGULAR,
    },
    Entry {
        stem: "Deseret",
        serif: false,
        postscript_name: "NotoSansDeseret",
        font: NOTOSANSDESERET_REGULAR,
    },
    Entry {
        stem: "Duployan",
        serif: false,
        postscript_name: "NotoSansDuployan",
        font: NOTOSANSDUPLOYAN_REGULAR,
    },
    Entry {
        stem: "EgyptianHieroglyphs",
        serif: false,
        postscript_name: "NotoSansEgyptianHieroglyphs",
        font: NOTOSANSEGYPTIANHIEROGLYPHS_REGULAR,
    },
    Entry {
        stem: "Elbasan",
        serif: false,
        postscript_name: "NotoSansElbasan",
        font: NOTOSANSELBASAN_REGULAR,
    },
    Entry {
        stem: "Elymaic",
        serif: false,
        postscript_name: "NotoSansElymaic",
        font: NOTOSANSELYMAIC_REGULAR,
    },
    Entry {
        stem: "Glagolitic",
        serif: false,
        postscript_name: "NotoSansGlagolitic",
        font: NOTOSANSGLAGOLITIC_REGULAR,
    },
    Entry {
        stem: "Gothic",
        serif: false,
        postscript_name: "NotoSansGothic",
        font: NOTOSANSGOTHIC_REGULAR,
    },
    Entry {
        stem: "GunjalaGondi",
        serif: false,
        postscript_name: "NotoSansGunjalaGondi",
        font: NOTOSANSGUNJALAGONDI_REGULAR,
    },
    Entry {
        stem: "HanifiRohingya",
        serif: false,
        postscript_name: "NotoSansHanifiRohingya",
        font: NOTOSANSHANIFIROHINGYA_REGULAR,
    },
    Entry {
        stem: "Hanunoo",
        serif: false,
        postscript_name: "NotoSansHanunoo",
        font: NOTOSANSHANUNOO_REGULAR,
    },
    Entry {
        stem: "Hatran",
        serif: false,
        postscript_name: "NotoSansHatran",
        font: NOTOSANSHATRAN_REGULAR,
    },
    Entry {
        stem: "ImperialAramaic",
        serif: false,
        postscript_name: "NotoSansImperialAramaic",
        font: NOTOSANSIMPERIALARAMAIC_REGULAR,
    },
    Entry {
        stem: "InscriptionalPahlavi",
        serif: false,
        postscript_name: "NotoSansInscriptionalPahlavi",
        font: NOTOSANSINSCRIPTIONALPAHLAVI_REGULAR,
    },
    Entry {
        stem: "InscriptionalParthian",
        serif: false,
        postscript_name: "NotoSansInscriptionalParthian",
        font: NOTOSANSINSCRIPTIONALPARTHIAN_REGULAR,
    },
    Entry {
        stem: "Javanese",
        serif: false,
        postscript_name: "NotoSansJavanese",
        font: NOTOSANSJAVANESE_REGULAR,
    },
    Entry {
        stem: "Kaithi",
        serif: false,
        postscript_name: "NotoSansKaithi",
        font: NOTOSANSKAITHI_REGULAR,
    },
    Entry {
        stem: "Kawi",
        serif: false,
        postscript_name: "NotoSansKawi",
        font: NOTOSANSKAWI_REGULAR,
    },
    Entry {
        stem: "KayahLi",
        serif: false,
        postscript_name: "NotoSansKayahLi",
        font: NOTOSANSKAYAHLI_REGULAR,
    },
    Entry {
        stem: "Kharoshthi",
        serif: false,
        postscript_name: "NotoSansKharoshthi",
        font: NOTOSANSKHAROSHTHI_REGULAR,
    },
    Entry {
        stem: "Khudawadi",
        serif: false,
        postscript_name: "NotoSansKhudawadi",
        font: NOTOSANSKHUDAWADI_REGULAR,
    },
    Entry {
        stem: "Lepcha",
        serif: false,
        postscript_name: "NotoSansLepcha",
        font: NOTOSANSLEPCHA_REGULAR,
    },
    Entry {
        stem: "Limbu",
        serif: false,
        postscript_name: "NotoSansLimbu",
        font: NOTOSANSLIMBU_REGULAR,
    },
    Entry {
        stem: "LinearA",
        serif: false,
        postscript_name: "NotoSansLinearA",
        font: NOTOSANSLINEARA_REGULAR,
    },
    Entry {
        stem: "LinearB",
        serif: false,
        postscript_name: "NotoSansLinearB",
        font: NOTOSANSLINEARB_REGULAR,
    },
    Entry {
        stem: "Lisu",
        serif: false,
        postscript_name: "NotoSansLisu",
        font: NOTOSANSLISU_REGULAR,
    },
    Entry {
        stem: "Lycian",
        serif: false,
        postscript_name: "NotoSansLycian",
        font: NOTOSANSLYCIAN_REGULAR,
    },
    Entry {
        stem: "Lydian",
        serif: false,
        postscript_name: "NotoSansLydian",
        font: NOTOSANSLYDIAN_REGULAR,
    },
    Entry {
        stem: "Mahajani",
        serif: false,
        postscript_name: "NotoSansMahajani",
        font: NOTOSANSMAHAJANI_REGULAR,
    },
    Entry {
        stem: "Mandaic",
        serif: false,
        postscript_name: "NotoSansMandaic",
        font: NOTOSANSMANDAIC_REGULAR,
    },
    Entry {
        stem: "Manichaean",
        serif: false,
        postscript_name: "NotoSansManichaean",
        font: NOTOSANSMANICHAEAN_REGULAR,
    },
    Entry {
        stem: "Marchen",
        serif: false,
        postscript_name: "NotoSansMarchen",
        font: NOTOSANSMARCHEN_REGULAR,
    },
    Entry {
        stem: "MasaramGondi",
        serif: false,
        postscript_name: "NotoSansMasaramGondi",
        font: NOTOSANSMASARAMGONDI_REGULAR,
    },
    Entry {
        stem: "Math",
        serif: false,
        postscript_name: "NotoSansMath",
        font: NOTOSANSMATH_REGULAR,
    },
    Entry {
        stem: "Medefaidrin",
        serif: false,
        postscript_name: "NotoSansMedefaidrin",
        font: NOTOSANSMEDEFAIDRIN_REGULAR,
    },
    Entry {
        stem: "MeeteiMayek",
        serif: false,
        postscript_name: "NotoSansMeeteiMayek",
        font: NOTOSANSMEETEIMAYEK_REGULAR,
    },
    Entry {
        stem: "MendeKikakui",
        serif: false,
        postscript_name: "NotoSansMendeKikakui",
        font: NOTOSANSMENDEKIKAKUI_REGULAR,
    },
    Entry {
        stem: "Meroitic",
        serif: false,
        postscript_name: "NotoSansMeroitic",
        font: NOTOSANSMEROITIC_REGULAR,
    },
    Entry {
        stem: "Miao",
        serif: false,
        postscript_name: "NotoSansMiao",
        font: NOTOSANSMIAO_REGULAR,
    },
    Entry {
        stem: "Modi",
        serif: false,
        postscript_name: "NotoSansModi",
        font: NOTOSANSMODI_REGULAR,
    },
    Entry {
        stem: "Mongolian",
        serif: false,
        postscript_name: "NotoSansMongolian",
        font: NOTOSANSMONGOLIAN_REGULAR,
    },
    Entry {
        stem: "Mro",
        serif: false,
        postscript_name: "NotoSansMro",
        font: NOTOSANSMRO_REGULAR,
    },
    Entry {
        stem: "Multani",
        serif: false,
        postscript_name: "NotoSansMultani",
        font: NOTOSANSMULTANI_REGULAR,
    },
    Entry {
        stem: "NKo",
        serif: false,
        postscript_name: "NotoSansNKo",
        font: NOTOSANSNKO_REGULAR,
    },
    Entry {
        stem: "Nabataean",
        serif: false,
        postscript_name: "NotoSansNabataean",
        font: NOTOSANSNABATAEAN_REGULAR,
    },
    Entry {
        stem: "NagMundari",
        serif: false,
        postscript_name: "NotoSansNagMundari",
        font: NOTOSANSNAGMUNDARI_REGULAR,
    },
    Entry {
        stem: "Nandinagari",
        serif: false,
        postscript_name: "NotoSansNandinagari",
        font: NOTOSANSNANDINAGARI_REGULAR,
    },
    Entry {
        stem: "NewTaiLue",
        serif: false,
        postscript_name: "NotoSansNewTaiLue",
        font: NOTOSANSNEWTAILUE_REGULAR,
    },
    Entry {
        stem: "Newa",
        serif: false,
        postscript_name: "NotoSansNewa",
        font: NOTOSANSNEWA_REGULAR,
    },
    Entry {
        stem: "Nushu",
        serif: false,
        postscript_name: "NotoSansNushu",
        font: NOTOSANSNUSHU_REGULAR,
    },
    Entry {
        stem: "Ogham",
        serif: false,
        postscript_name: "NotoSansOgham",
        font: NOTOSANSOGHAM_REGULAR,
    },
    Entry {
        stem: "OlChiki",
        serif: false,
        postscript_name: "NotoSansOlChiki",
        font: NOTOSANSOLCHIKI_REGULAR,
    },
    Entry {
        stem: "OldHungarian",
        serif: false,
        postscript_name: "NotoSansOldHungarian",
        font: NOTOSANSOLDHUNGARIAN_REGULAR,
    },
    Entry {
        stem: "OldItalic",
        serif: false,
        postscript_name: "NotoSansOldItalic",
        font: NOTOSANSOLDITALIC_REGULAR,
    },
    Entry {
        stem: "OldNorthArabian",
        serif: false,
        postscript_name: "NotoSansOldNorthArabian",
        font: NOTOSANSOLDNORTHARABIAN_REGULAR,
    },
    Entry {
        stem: "OldPermic",
        serif: false,
        postscript_name: "NotoSansOldPermic",
        font: NOTOSANSOLDPERMIC_REGULAR,
    },
    Entry {
        stem: "OldPersian",
        serif: false,
        postscript_name: "NotoSansOldPersian",
        font: NOTOSANSOLDPERSIAN_REGULAR,
    },
    Entry {
        stem: "OldSogdian",
        serif: false,
        postscript_name: "NotoSansOldSogdian",
        font: NOTOSANSOLDSOGDIAN_REGULAR,
    },
    Entry {
        stem: "OldSouthArabian",
        serif: false,
        postscript_name: "NotoSansOldSouthArabian",
        font: NOTOSANSOLDSOUTHARABIAN_REGULAR,
    },
    Entry {
        stem: "OldTurkic",
        serif: false,
        postscript_name: "NotoSansOldTurkic",
        font: NOTOSANSOLDTURKIC_REGULAR,
    },
    Entry {
        stem: "Osage",
        serif: false,
        postscript_name: "NotoSansOsage",
        font: NOTOSANSOSAGE_REGULAR,
    },
    Entry {
        stem: "Osmanya",
        serif: false,
        postscript_name: "NotoSansOsmanya",
        font: NOTOSANSOSMANYA_REGULAR,
    },
    Entry {
        stem: "PahawhHmong",
        serif: false,
        postscript_name: "NotoSansPahawhHmong",
        font: NOTOSANSPAHAWHHMONG_REGULAR,
    },
    Entry {
        stem: "Palmyrene",
        serif: false,
        postscript_name: "NotoSansPalmyrene",
        font: NOTOSANSPALMYRENE_REGULAR,
    },
    Entry {
        stem: "PauCinHau",
        serif: false,
        postscript_name: "NotoSansPauCinHau",
        font: NOTOSANSPAUCINHAU_REGULAR,
    },
    Entry {
        stem: "PhagsPa",
        serif: false,
        postscript_name: "NotoSansPhagsPa",
        font: NOTOSANSPHAGSPA_REGULAR,
    },
    Entry {
        stem: "Phoenician",
        serif: false,
        postscript_name: "NotoSansPhoenician",
        font: NOTOSANSPHOENICIAN_REGULAR,
    },
    Entry {
        stem: "PsalterPahlavi",
        serif: false,
        postscript_name: "NotoSansPsalterPahlavi",
        font: NOTOSANSPSALTERPAHLAVI_REGULAR,
    },
    Entry {
        stem: "Rejang",
        serif: false,
        postscript_name: "NotoSansRejang",
        font: NOTOSANSREJANG_REGULAR,
    },
    Entry {
        stem: "Runic",
        serif: false,
        postscript_name: "NotoSansRunic",
        font: NOTOSANSRUNIC_REGULAR,
    },
    Entry {
        stem: "Samaritan",
        serif: false,
        postscript_name: "NotoSansSamaritan",
        font: NOTOSANSSAMARITAN_REGULAR,
    },
    Entry {
        stem: "Saurashtra",
        serif: false,
        postscript_name: "NotoSansSaurashtra",
        font: NOTOSANSSAURASHTRA_REGULAR,
    },
    Entry {
        stem: "Sharada",
        serif: false,
        postscript_name: "NotoSansSharada",
        font: NOTOSANSSHARADA_REGULAR,
    },
    Entry {
        stem: "Shavian",
        serif: false,
        postscript_name: "NotoSansShavian",
        font: NOTOSANSSHAVIAN_REGULAR,
    },
    Entry {
        stem: "Siddham",
        serif: false,
        postscript_name: "NotoSansSiddham",
        font: NOTOSANSSIDDHAM_REGULAR,
    },
    Entry {
        stem: "SignWriting",
        serif: false,
        postscript_name: "NotoSansSignWriting",
        font: NOTOSANSSIGNWRITING_REGULAR,
    },
    Entry {
        stem: "Sogdian",
        serif: false,
        postscript_name: "NotoSansSogdian",
        font: NOTOSANSSOGDIAN_REGULAR,
    },
    Entry {
        stem: "SoraSompeng",
        serif: false,
        postscript_name: "NotoSansSoraSompeng",
        font: NOTOSANSSORASOMPENG_REGULAR,
    },
    Entry {
        stem: "Soyombo",
        serif: false,
        postscript_name: "NotoSansSoyombo",
        font: NOTOSANSSOYOMBO_REGULAR,
    },
    Entry {
        stem: "Sundanese",
        serif: false,
        postscript_name: "NotoSansSundanese",
        font: NOTOSANSSUNDANESE_REGULAR,
    },
    Entry {
        stem: "SylotiNagri",
        serif: false,
        postscript_name: "NotoSansSylotiNagri",
        font: NOTOSANSSYLOTINAGRI_REGULAR,
    },
    Entry {
        stem: "Symbols",
        serif: false,
        postscript_name: "NotoSansSymbols",
        font: NOTOSANSSYMBOLS_REGULAR,
    },
    Entry {
        stem: "Symbols2",
        serif: false,
        postscript_name: "NotoSansSymbols2",
        font: NOTOSANSSYMBOLS2_REGULAR,
    },
    Entry {
        stem: "Syriac",
        serif: false,
        postscript_name: "NotoSansSyriac",
        font: NOTOSANSSYRIAC_REGULAR,
    },
    Entry {
        stem: "Tagalog",
        serif: false,
        postscript_name: "NotoSansTagalog",
        font: NOTOSANSTAGALOG_REGULAR,
    },
    Entry {
        stem: "Tagbanwa",
        serif: false,
        postscript_name: "NotoSansTagbanwa",
        font: NOTOSANSTAGBANWA_REGULAR,
    },
    Entry {
        stem: "TaiLe",
        serif: false,
        postscript_name: "NotoSansTaiLe",
        font: NOTOSANSTAILE_REGULAR,
    },
    Entry {
        stem: "TaiTham",
        serif: false,
        postscript_name: "NotoSansTaiTham",
        font: NOTOSANSTAITHAM_REGULAR,
    },
    Entry {
        stem: "TaiViet",
        serif: false,
        postscript_name: "NotoSansTaiViet",
        font: NOTOSANSTAIVIET_REGULAR,
    },
    Entry {
        stem: "Takri",
        serif: false,
        postscript_name: "NotoSansTakri",
        font: NOTOSANSTAKRI_REGULAR,
    },
    Entry {
        stem: "Tangsa",
        serif: false,
        postscript_name: "NotoSansTangsa",
        font: NOTOSANSTANGSA_REGULAR,
    },
    Entry {
        stem: "Thaana",
        serif: false,
        postscript_name: "NotoSansThaana",
        font: NOTOSANSTHAANA_REGULAR,
    },
    Entry {
        stem: "Tifinagh",
        serif: false,
        postscript_name: "NotoSansTifinagh",
        font: NOTOSANSTIFINAGH_REGULAR,
    },
    Entry {
        stem: "Tirhuta",
        serif: false,
        postscript_name: "NotoSansTirhuta",
        font: NOTOSANSTIRHUTA_REGULAR,
    },
    Entry {
        stem: "Ugaritic",
        serif: false,
        postscript_name: "NotoSansUgaritic",
        font: NOTOSANSUGARITIC_REGULAR,
    },
    Entry {
        stem: "Vai",
        serif: false,
        postscript_name: "NotoSansVai",
        font: NOTOSANSVAI_REGULAR,
    },
    Entry {
        stem: "Wancho",
        serif: false,
        postscript_name: "NotoSansWancho",
        font: NOTOSANSWANCHO_REGULAR,
    },
    Entry {
        stem: "WarangCiti",
        serif: false,
        postscript_name: "NotoSansWarangCiti",
        font: NOTOSANSWARANGCITI_REGULAR,
    },
    Entry {
        stem: "Yi",
        serif: false,
        postscript_name: "NotoSansYi",
        font: NOTOSANSYI_REGULAR,
    },
    Entry {
        stem: "ZanabazarSquare",
        serif: false,
        postscript_name: "NotoSansZanabazarSquare",
        font: NOTOSANSZANABAZARSQUARE_REGULAR,
    },
    Entry {
        stem: "",
        serif: true,
        postscript_name: "NotoSerif",
        font: NOTOSERIF_REGULAR,
    },
    Entry {
        stem: "Ahom",
        serif: true,
        postscript_name: "NotoSerifAhom",
        font: NOTOSERIFAHOM_REGULAR,
    },
    Entry {
        stem: "Armenian",
        serif: true,
        postscript_name: "NotoSerifArmenian",
        font: NOTOSERIFARMENIAN_REGULAR,
    },
    Entry {
        stem: "Balinese",
        serif: true,
        postscript_name: "NotoSerifBalinese",
        font: NOTOSERIFBALINESE_REGULAR,
    },
    Entry {
        stem: "Bengali",
        serif: true,
        postscript_name: "NotoSerifBengali",
        font: NOTOSERIFBENGALI_REGULAR,
    },
    Entry {
        stem: "Devanagari",
        serif: true,
        postscript_name: "NotoSerifDevanagari",
        font: NOTOSERIFDEVANAGARI_REGULAR,
    },
    Entry {
        stem: "DivesAkuru",
        serif: true,
        postscript_name: "NotoSerifDivesAkuru",
        font: NOTOSERIFDIVESAKURU_REGULAR,
    },
    Entry {
        stem: "Dogra",
        serif: true,
        postscript_name: "NotoSerifDogra",
        font: NOTOSERIFDOGRA_REGULAR,
    },
    Entry {
        stem: "Ethiopic",
        serif: true,
        postscript_name: "NotoSerifEthiopic",
        font: NOTOSERIFETHIOPIC_REGULAR,
    },
    Entry {
        stem: "Georgian",
        serif: true,
        postscript_name: "NotoSerifGeorgian",
        font: NOTOSERIFGEORGIAN_REGULAR,
    },
    Entry {
        stem: "Grantha",
        serif: true,
        postscript_name: "NotoSerifGrantha",
        font: NOTOSERIFGRANTHA_REGULAR,
    },
    Entry {
        stem: "Gujarati",
        serif: true,
        postscript_name: "NotoSerifGujarati",
        font: NOTOSERIFGUJARATI_REGULAR,
    },
    Entry {
        stem: "Gurmukhi",
        serif: true,
        postscript_name: "NotoSerifGurmukhi",
        font: NOTOSERIFGURMUKHI_REGULAR,
    },
    Entry {
        stem: "Hebrew",
        serif: true,
        postscript_name: "NotoSerifHebrew",
        font: NOTOSERIFHEBREW_REGULAR,
    },
    Entry {
        stem: "Kannada",
        serif: true,
        postscript_name: "NotoSerifKannada",
        font: NOTOSERIFKANNADA_REGULAR,
    },
    Entry {
        stem: "KhitanSmallScript",
        serif: true,
        postscript_name: "NotoSerifKhitanSmallScript",
        font: NOTOSERIFKHITANSMALLSCRIPT_REGULAR,
    },
    Entry {
        stem: "Khmer",
        serif: true,
        postscript_name: "NotoSerifKhmer",
        font: NOTOSERIFKHMER_REGULAR,
    },
    Entry {
        stem: "Khojki",
        serif: true,
        postscript_name: "NotoSerifKhojki",
        font: NOTOSERIFKHOJKI_REGULAR,
    },
    Entry {
        stem: "Lao",
        serif: true,
        postscript_name: "NotoSerifLao",
        font: NOTOSERIFLAO_REGULAR,
    },
    Entry {
        stem: "Makasar",
        serif: true,
        postscript_name: "NotoSerifMakasar",
        font: NOTOSERIFMAKASAR_REGULAR,
    },
    Entry {
        stem: "Malayalam",
        serif: true,
        postscript_name: "NotoSerifMalayalam",
        font: NOTOSERIFMALAYALAM_REGULAR,
    },
    Entry {
        stem: "Myanmar",
        serif: true,
        postscript_name: "NotoSerifMyanmar",
        font: NOTOSERIFMYANMAR_REGULAR,
    },
    Entry {
        stem: "NyiakengPuachueHmong",
        serif: true,
        postscript_name: "NotoSerifNyiakengPuachueHmong",
        font: NOTOSERIFNYIAKENGPUACHUEHMONG_REGULAR,
    },
    Entry {
        stem: "OldUyghur",
        serif: true,
        postscript_name: "NotoSerifOldUyghur",
        font: NOTOSERIFOLDUYGHUR_REGULAR,
    },
    Entry {
        stem: "Oriya",
        serif: true,
        postscript_name: "NotoSerifOriya",
        font: NOTOSERIFORIYA_REGULAR,
    },
    Entry {
        stem: "Sinhala",
        serif: true,
        postscript_name: "NotoSerifSinhala",
        font: NOTOSERIFSINHALA_REGULAR,
    },
    Entry {
        stem: "Tamil",
        serif: true,
        postscript_name: "NotoSerifTamil",
        font: NOTOSERIFTAMIL_REGULAR,
    },
    Entry {
        stem: "Tangut",
        serif: true,
        postscript_name: "NotoSerifTangut",
        font: NOTOSERIFTANGUT_REGULAR,
    },
    Entry {
        stem: "Telugu",
        serif: true,
        postscript_name: "NotoSerifTelugu",
        font: NOTOSERIFTELUGU_REGULAR,
    },
    Entry {
        stem: "Thai",
        serif: true,
        postscript_name: "NotoSerifThai",
        font: NOTOSERIFTHAI_REGULAR,
    },
    Entry {
        stem: "Tibetan",
        serif: true,
        postscript_name: "NotoSerifTibetan",
        font: NOTOSERIFTIBETAN_REGULAR,
    },
    Entry {
        stem: "Toto",
        serif: true,
        postscript_name: "NotoSerifToto",
        font: NOTOSERIFTOTO_REGULAR,
    },
    Entry {
        stem: "Vithkuqi",
        serif: true,
        postscript_name: "NotoSerifVithkuqi",
        font: NOTOSERIFVITHKUQI_REGULAR,
    },
    Entry {
        stem: "Yezidi",
        serif: true,
        postscript_name: "NotoSerifYezidi",
        font: NOTOSERIFYEZIDI_REGULAR,
    },
];

pub fn find_by_name(name: &str, bold: bool, italic: bool) -> Option<Font> {
    if bold || italic {
        return None;
    }

    FONTS.iter().find_map(|entry| {
        if eq_font_name(name, entry.font.name) || eq_font_name(name, entry.postscript_name) {
            Some(entry.font)
        } else {
            None
        }
    })
}

pub fn find_by_stem(stem: &str, serif: bool) -> Option<Font> {
    let mut fallback = None;

    for entry in FONTS {
        if !entry.stem.eq_ignore_ascii_case(stem) {
            continue;
        }
        if entry.serif == serif {
            return Some(entry.font);
        }
        fallback.get_or_insert(entry.font);
    }

    fallback
}

fn eq_font_name(a: &str, b: &str) -> bool {
    let mut a = normalized_bytes(a);
    let mut b = normalized_bytes(b);

    loop {
        match (a.next(), b.next()) {
            (None, None) => return true,
            (Some(x), Some(y)) if x == y => {}
            _ => return false,
        }
    }
}

fn normalized_bytes(s: &str) -> impl Iterator<Item = u8> + '_ {
    s.bytes()
        .filter(|b| b.is_ascii_alphanumeric())
        .map(|b| b.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_fonts_by_name_and_stem() {
        assert_eq!(
            find_by_name("Noto Sans Modi", false, false).unwrap().name,
            "Noto Sans Modi"
        );
        assert_eq!(
            find_by_name("NotoSansModi", false, false).unwrap().name,
            "Noto Sans Modi"
        );
        assert_eq!(find_by_stem("Modi", false).unwrap().name, "Noto Sans Modi");
        assert_eq!(
            find_by_stem("Bengali", true).unwrap().name,
            "Noto Serif Bengali"
        );
        assert_eq!(
            find_by_stem("Naskh", false).unwrap().name,
            "Noto Naskh Arabic"
        );
        assert!(find_by_name("Noto Sans Modi", true, false).is_none());
    }
}
