"""Builds data/scan/names_zh.json: each card's English name (as names.json has it) -> the Chinese names the
reader accepts for it. Sources: Moebing's Torchlight-Infinite-SS11-Vorax-simulator (MIT, Simplified Chinese, the
same order as names.json), OpenCC's Traditional forms, and names read off a Traditional Chinese client.

    python3 dev/make_names_zh.py <moebing source> data/scan/names.json data/scan/names_zh.json
"""
import json, re, sys
import opencc

moebing, names_path, out = sys.argv[1:4]
src = open(moebing, encoding='utf-8').read()
zh = re.findall(r"name: '([^']*)'", src)
en = json.load(open(names_path, encoding='utf-8'))
s2t, s2tw = opencc.OpenCC('s2t'), opencc.OpenCC('s2tw')

pairs = {'potion': {}, 'instrument': {}, 'protocol': {}}
# potions 1-33 and instruments 1-24 are in the same order in both projects
for e, z in zip(en['potion'][:33], zh[:33]):
    pairs['potion'][e] = [z]
kits = {'Potion Kit (Large)': '药剂箱（大）', 'Rare Potion Kit (Small)': '稀有药剂箱', 'Potion Kit (Small)': '药剂箱（小）',
        'Compound Revitalizing Pill': '复方焕生丸剂', 'Fast-Acting Heart Tonic': '速效强心剂',
        'Plague Quarter Peat Poultice': '疫区泥炭敷料', 'Insect Attractant': '诱虫剂', 'Purest Holy Water': '至纯圣水'}
for e, z in kits.items():
    assert e in en['potion'], e
    pairs['potion'][e] = [z]
for e, z in zip(en['instrument'][:24], zh[41:65]):
    pairs['instrument'][e] = [z]

# as a Traditional Chinese client prints them (read off its screen, Nov 2026)
seen = {
    'instrument': {'Chrysalis Specimen': '人蛹標本', 'Hyperplastic Frontal Lobe': '增生前額葉', 'Hypertrophic Pituitary': '腫大腦垂體',
                   'Adherent Metatarsals': '黏連跖骨', 'Contracted Claws': '攣縮指爪', 'Aberrant Sprout': '孽生肉芽',
                   'Prismatic Liver': '斑斕肝臟', 'Sinew Garrote': '獸筋絞肉索', 'Vermicular Spinal Cord': '蠕動脊髓',
                   'Fouled Scraper': '髒污刮骨刀', 'Canine Teeth File': '犬牙銼刀', 'Eyelid Speculum': '眼瞼擴張器',
                   'Incubation Sac': '孵化囊', 'Clustered Eggs': '簇生蟲卵', 'Sloughed Cortex': '蛻生腦皮層',
                   'Rawhide Restraint': '生皮革拘束帶', 'Black Goat Gut Suture': '黑山羊腸縫線', 'Pig Iron Bone Saw': '生鐵骨鋸',
                   'Two-Inch Skull Nail': '二寸顱骨釘', 'Lady of the Plague Quarter': '疫區聖母像', 'Aberrant Stinger': '異形螫針',
                   'Tanned Leather Restraint': '鞣製皮革拘束帶', 'Beak of the Twice-Born': '二度降生者之喙', 'Nettle Toggle': '蕁麻繩扣',
                   'Egg-Burdened Butterfly': '抱卵蝶'},
    'potion': {'Slough Solution': '蛻生皮溶液', 'Cleansing Ointment': '清疽油膏', 'Viviparous Hormone': '活性育卵激素',
               'Eggshell Powder': '卵殼藥粉', 'Brain Fog Tincture': '腦霧酊劑', 'Osteogenic Powder': '生骨藥粉',
               'Hollow Spinal Solution': '空心脊髓溶液', 'Green Bile Solution': '青膽汁溶液', 'Digestive Enzyme Solution': '消化酶溶液',
               'Vivigenic Powder': '活殖藥粉', 'Xenopurge Powder': '祛異藥粉', 'Bewitching Tincture': '迷魂酊劑',
               'Mutagenic Powder': '誘變藥粉', 'Slender-Limb Powder - Pest': '細肢藥粉-蠱蟲', 'Twin Hormone - Pest': '孿生激素-蠱蟲',
               'Spinal Solution - Agito': '脊髓溶液-覺醒者', 'Anesthetic Tincture - Agito': '麻藥酊劑-覺醒者',
               'Slender-Limb Powder - Bone Sentry': '細肢藥粉-骨衛兵', 'Twin Hormone - Bone Sentry': '孿生激素-骨衛兵',
               'Anesthetic Tincture - Aemberon': '麻藥酊劑-異魔', 'Spinal Solution - Aemberon': '脊髓溶液-異魔',
               'Xeno-Hormone': '異種激素', 'Targeted Xeno-Hormone': '靶向異種激素', 'Potion Kit (Small)': '藥劑箱（小）',
               'Rare Potion Kit (Small)': '稀有藥劑箱（小）', 'Potion Kit (Large)': '藥劑箱（大）',
               'Gray Matter Spinal Solution': '脊髓灰質溶液', 'Probiotic Enzyme Solution': '益生黴溶液',
               'Leptomeningeal Solution': '軟腦膜溶液', 'Fresh Spinal Powder': '鮮脊髓藥粉',
               'Petrified Spinal Solution': '石化脊髓溶液', 'Osteolytic Ointment': '化骨油膏',
               'Potent Xenopurge Powder': '強效祛異藥粉', 'Viscous Bile Solution': '黏稠膽汁溶液',
               'Compound Leech Solution': '混合活蛭溶液', 'Pure Leech Solution': '純粹活蛭溶液',
               'Fast-Acting Heart Tonic': '速效強心劑', 'Compound Revitalizing Pill': '複方煥生丸劑',
               'Plague Quarter Peat Poultice': '疫區泥炭敷料', 'Rousing Salts': '驚醒嗅鹽',
               'Insect Attractant': '誘蟲劑', 'Purest Holy Water': '至純聖水',
               'Xenopanacea: Osteotrophy': '異種萬靈藥：壯骨', 'Xenopanacea: Brood': '異種萬靈藥：育雛',
               'Xenopanacea': '異種萬靈藥',
               'First Rite Holy Water: Confession': '第一儀式聖水：告解',
               # the data has one more rite; the Solver takes any rite when offered, so the others stand in for it
               'Ritual Holy Water: Flagellation': '第二儀式聖水：鞭刑'},
    'protocol': {'Trepanation': '顱骨鑽孔術', 'Spinal Cord Stimulation': '脊髓神經刺激術', 'Anesthesia Analgesia': '麻醉止痛法',
                 'Muscle Relaxant Injection Experiment': '肌肉鬆弛劑注射實驗', 'Cyst Rupture Therapy': '囊腫爆破療法',
                 'Laparolithotomy': '剖腹取石療法', 'Yellow Bile Therapy': '黑膽汁療法', 'Laser Amputation': '雷射截肢手術',
                 'Frontal Lobe Repair': '額葉修復術', 'Live Leech Therapy': '活蛭吸附療法',
                 # the only experiment left among the English names; protocols are only read to know the kind of pick
                 'Immunosuppressive Experiment': '表皮移植實驗', 'Holistic Acupuncture Therapy': '全身針灸療法',
                 # the last protocol left among the English names; protocols are only read to know the kind of pick
                 'Aseptic Isolation Protocol': '生命復甦術'},
}
extra = {'potion': {'Ritual Holy Water: Flagellation': ['第三儀式聖水：啟蒙', '第四儀式聖水：聖餐']}}
for phase, m in extra.items():
    for e, zs in m.items():
        pairs[phase].setdefault(e, []).extend(zs)
for phase, m in seen.items():
    for e, z in m.items():
        assert e in en[phase], (phase, e)
        pairs[phase].setdefault(e, []).insert(0, z)

out_data = {}
for phase in ['potion', 'instrument', 'protocol']:
    out_data[phase] = {}
    for e in en[phase]:
        if e not in pairs[phase]:
            continue
        forms = []
        for z in pairs[phase][e]:
            for f in (z, s2t.convert(z), s2tw.convert(z)):
                f = f.replace(' ', '')
                if f not in forms:
                    forms.append(f)
        out_data[phase][e] = forms
json.dump(out_data, open(out, 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
for phase in out_data:
    missing = [e for e in en[phase] if e not in out_data[phase]]
    print(phase, len(out_data[phase]), '/', len(en[phase]), 'missing:', missing)

# a character fold for matching: Traditional (and variant) characters to Simplified, so a read that mixes the two
# still matches
t2s = opencc.OpenCC('t2s')
chars = set()
for phase in ['potion', 'instrument', 'protocol']:
    for forms in out_data[phase].values():
        for f in forms:
            chars.update(f)
fold = {}
for c in sorted(chars):
    s = t2s.convert(c)
    if s != c and len(s) == 1:
        fold[c] = s
# 痳: Windows' reader takes 麻 for it (1920x1080)
fold.update({'黏': '粘', '蹠': '跖', '鬆': '松', '衛': '卫', '痳': '麻'})
out_data['fold'] = fold
# cards a Chinese client shows that upstream's data doesn't have (the Solver can't weigh them): recognized so the
# hint says so, instead of "not read"
out_data['unsupported'] = ['解剖浸液：病軀', '解剖浸液：病躯']
json.dump(out_data, open(out, 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
